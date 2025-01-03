const std = @import("std");
const builtin = @import("builtin");

fn compareFileNames(_: void, lhs: std.fs.Dir.Entry, rhs: std.fs.Dir.Entry) bool {
    return std.mem.order(u8, lhs.name, rhs.name).compare(std.math.CompareOperator.lt);
}

fn processOutput(allocator: std.mem.Allocator, input: []const u8) ![]u8 {
    var result = std.ArrayList(u8).init(allocator);
    var i: usize = 0;

    while (i < input.len) {
        if (input[i] == '\x08' and result.items.len > 0) {
            _ = result.pop();
        } else if (input[i] != '\x08') {
            try result.append(input[i]);
        }
        i += 1;
    }

    return result.toOwnedSlice();
}

const FEntry = struct {
    extension: []const u8,
    code: []const u8,
    index: usize,
    output: []const u8,
};

pub fn main() !void {
    // Get command line arguments
    const allocator = std.heap.page_allocator;
    var args_it = std.process.argsWithAllocator(allocator) catch {
        std.debug.print("Failed to get command line arguments\n", .{});
        std.process.exit(1);
    };
    defer args_it.deinit();

    // Skip the program name
    _ = args_it.next();

    // Get the extension
    const extension_arg = args_it.next() orelse {
        std.debug.print("Usage: <program> <extension> <folder>\n", .{});
        std.process.exit(1);
    };

    // Get the directory path
    const dir_path = args_it.next() orelse {
        std.debug.print("Usage: <program> <extension> <folder>\n", .{});
        std.process.exit(1);
    };

    const env_map = std.process.getEnvMap(allocator) catch {
        std.debug.print("Failed to get environment variables\n", .{});
        std.process.exit(1);
    };

    const home = env_map.get("HOME") orelse "";

    const full_dir_path = if (std.fs.path.isAbsolute(dir_path))
        try std.mem.concat(allocator, u8, &[_][]const u8{ dir_path, "/" })
    else
        std.mem.concat(allocator, u8, &[_][]const u8{ home, "/", dir_path, "/" }) catch {
            std.debug.print("Failed to construct directory path\n", .{});
            std.process.exit(1);
        };

    defer allocator.free(full_dir_path);
    const questions_file = try std.fs.path.join(allocator, &[_][]const u8{ full_dir_path, "questions.txt" });
    defer allocator.free(questions_file);

    const raw_questions = std.fs.cwd().readFileAlloc(allocator, questions_file, std.math.maxInt(usize)) catch {
        std.debug.print("Failed to read questions file\n", .{});
        std.process.exit(1);
    };

    defer allocator.free(raw_questions);
    var questions = std.mem.split(u8, raw_questions, "\n---\n");
    var qal = std.ArrayList([]const u8).init(allocator);
    defer qal.deinit();

    while (questions.next()) |question| {
        qal.append(question) catch {
            std.debug.print("Failed to append question\n", .{});
            std.process.exit(1);
        };
    }

    var dir_entries = std.ArrayList(std.fs.Dir.Entry).init(std.heap.page_allocator);
    defer dir_entries.deinit();

    var dir = std.fs.cwd().openDir(full_dir_path, .{}) catch {
        std.debug.print("Failed to open directory\n", .{});
        std.process.exit(1);
    };
    defer dir.close();

    var dir_it = dir.iterate();
    while (try dir_it.next()) |entry| {
        dir_entries.append(entry) catch {
            std.debug.print("Failed to append directory entry\n", .{});
            std.process.exit(1);
        };
    }

    std.mem.sort(std.fs.Dir.Entry, dir_entries.items, {}, compareFileNames);

    var entries = std.StringHashMap(FEntry).init(allocator);
    defer {
        var iter = entries.iterator();
        while (iter.next()) |entry| {
            allocator.free(entry.value_ptr.code);
        }
        entries.deinit();
    }

    const extension = try std.mem.concat(allocator, u8, &[_][]const u8{ ".", extension_arg });
    defer allocator.free(extension);

    var index: usize = 0;
    for (dir_entries.items) |entry| {
        if (entry.kind == .file and std.mem.endsWith(u8, entry.name, extension)) {
            if (index >= qal.items.len) {
                std.debug.print("Warning: More files than questions found!\n", .{});
                break;
            }

            std.debug.print("\n=== Processing file: {s} ===\n", .{entry.name});
            var file = dir.openFile(entry.name, .{}) catch {
                std.debug.print("Failed to open file: {s}\n", .{entry.name});
                std.process.exit(1);
            };
            const content = file.readToEndAlloc(allocator, std.math.maxInt(usize)) catch {
                std.debug.print("Failed to read file: {s}\n", .{entry.name});
                std.process.exit(1);
            };
            file.close();

            // Create a temporary file for output capture
            const tmp_output_path = "/tmp/terminal_output.txt";

            // Clear the output file
            {
                const tmp_file = try std.fs.cwd().createFile(tmp_output_path, .{});
                tmp_file.close();
                defer {
                    std.fs.cwd().deleteFile(tmp_output_path) catch {};
                }
            }

            var output: []const u8 = "";

            if (std.mem.eql(u8, extension, ".cpp")) {
                const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "/", entry.name });
                defer allocator.free(script_path);

                // Compile with explicit output handling
                {
                    var compile = std.process.Child.init(&[_][]const u8{ "g++", script_path, "-o", "./a.out" }, allocator);
                    compile.stdout_behavior = .Pipe;
                    compile.stderr_behavior = .Pipe;
                    compile.spawn() catch {
                        std.debug.print("Failed to compile file: {s}\n", .{entry.name});
                        std.process.exit(1);
                    };
                    _ = try compile.wait();
                }
                defer std.fs.cwd().deleteFile("./a.out") catch {};

                // Use script command with explicit file
                const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} ./a.out", .{tmp_output_path});
                defer allocator.free(shell_cmd);

                var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
                run_process.spawn() catch {
                    std.debug.print("Failed to run file: {s}\n", .{entry.name});
                    std.process.exit(1);
                };
                _ = try run_process.wait();

                // Read output file directly
                output = try processOutput(allocator, try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize)));
            } else if (std.mem.eql(u8, extension, ".py")) {
                const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "/", entry.name });
                defer allocator.free(script_path);
                const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} python -u {s}", .{ tmp_output_path, script_path });
                defer allocator.free(shell_cmd);

                var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
                run_process.spawn() catch {
                    std.debug.print("Failed to run file: {s}\n", .{entry.name});
                    std.process.exit(1);
                };
                _ = try run_process.wait();

                // Read output file directly
                output = try processOutput(allocator, try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize)));
            }

            try entries.put(qal.items[index], FEntry{
                .extension = extension,
                .code = try allocator.dupe(u8, content),
                .index = index,
                .output = output,
            });
            index += 1;
        }
    }

    // Write to json file with explicit flush
    var json_array = std.ArrayList(u8).init(allocator);
    defer json_array.deinit();

    const writer = json_array.writer();
    try writer.writeByte('[');

    var first = true;
    var iter = entries.iterator();
    while (iter.next()) |entry| {
        if (!first) {
            try writer.writeByte(',');
        }
        first = false;

        try writer.writeByte('\n');
        try writer.writeAll("  {");
        try writer.writeAll("\n    \"question\": ");
        try std.json.stringify(entry.key_ptr.*, .{}, writer);
        try writer.writeAll(",\n    \"index\": ");
        try std.json.stringify(entry.value_ptr.index, .{}, writer);
        try writer.writeAll(",\n    \"extension\": ");
        try std.json.stringify(entry.value_ptr.extension, .{}, writer);
        try writer.writeAll(",\n    \"code\": ");
        try std.json.stringify(entry.value_ptr.code, .{}, writer);
        try writer.writeAll(",\n    \"output\": ");
        try std.json.stringify(entry.value_ptr.output, .{}, writer);
        try writer.writeAll("\n  }");
    }

    if (!first) {
        try writer.writeByte('\n');
    }
    try writer.writeByte(']');

    // Write to file with explicit sync
    const file = try dir.createFile("output.json", .{});
    defer file.close();
    try file.writeAll(json_array.items);
    try file.sync();

    const rust_executable_path = if (builtin.mode == .Debug)
        "target/release/create-docx"
    else
        "/usr/local/bin/create-docx";

    var rust_process = std.process.Child.init(&[_][]const u8{ rust_executable_path, dir_path }, allocator);
    rust_process.stderr_behavior = .Pipe;
    rust_process.stdout_behavior = .Pipe;

    try rust_process.spawn();
    const result = try rust_process.wait();

    if (result != .Exited or result.Exited != 0) {
        const stderr = try rust_process.stderr.?.reader().readAllAlloc(allocator, 10_000);
        defer allocator.free(stderr);
        std.log.err("Rust process failed: {s}", .{stderr});
        return error.RustProcessFailed;
    }
}
