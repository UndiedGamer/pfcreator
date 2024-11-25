const std = @import("std");
const builtin = @import("builtin");

fn sortFiles(_: void, lhs: std.fs.Dir.Entry, rhs: std.fs.Dir.Entry) bool {
    return std.mem.order(u8, lhs.name, rhs.name).compare(std.math.CompareOperator.lt);
}

pub fn main() !void {
    const FEntry = struct {
        extension: []const u8,
        code: []const u8,
        index: usize,
        output: []const u8,
    };

    // Get command line arguments
    const allocator = std.heap.page_allocator;
    var args_it = std.process.args();
    defer args_it.deinit();

    // Skip the program name
    _ = args_it.next();

    // Get the extension
    const extension_arg = args_it.next() orelse {
        std.debug.print("Usage: <program> <extension> <folder>\n", .{});
        return;
    };

    // Get the directory path
    const dir_path = args_it.next() orelse {
        std.debug.print("Usage: <program> <extension> <folder>\n", .{});
        return;
    };

    const raw_questions = try std.fs.cwd().readFileAlloc(allocator, "./questions.txt", std.math.maxInt(usize));
    defer allocator.free(raw_questions);
    var questions = std.mem.split(u8, raw_questions, "\n---\n");
    var qal = std.ArrayList([]const u8).init(allocator);
    defer qal.deinit();

    while (questions.next()) |question| {
        try qal.append(question);
    }

    var dir_entries = std.ArrayList(std.fs.Dir.Entry).init(std.heap.page_allocator);
    defer dir_entries.deinit();

    var dir = try std.fs.cwd().openDir(dir_path, .{});
    defer dir.close();

    var dir_it = dir.iterate();
    while (try dir_it.next()) |entry| {
        try dir_entries.append(entry);
    }

    std.mem.sort(std.fs.Dir.Entry, dir_entries.items, {}, sortFiles);

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

    for (dir_entries.items, 0..) |entry, index| {
        if (entry.kind == .file and std.mem.endsWith(u8, entry.name, extension)) {
            if (index >= qal.items.len) {
                std.debug.print("Warning: More files than questions found!\n", .{});
                break;
            }

            std.debug.print("\n=== Processing file: {s} ===\n", .{entry.name});
            var file = try dir.openFile(entry.name, .{});
            const content = try file.readToEndAlloc(allocator, std.math.maxInt(usize));
            file.close();

            // Create a temporary file for output capture
            const tmp_output_path = if (builtin.os.tag == .windows)
                "temp_output.txt"
            else
                "/tmp/terminal_output.txt";

            // Clear the output file
            {
                const tmp_file = try std.fs.cwd().createFile(tmp_output_path, .{});
                tmp_file.close();
            }

            var output: []const u8 = "";

            if (std.mem.eql(u8, extension, ".cpp")) {
                const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ dir_path, "/", entry.name });
                defer allocator.free(script_path);

                // Compile with explicit output handling
                {
                    var compile = std.process.Child.init(&[_][]const u8{ "g++", script_path, "-o", "./a.out" }, allocator);
                    compile.stdout_behavior = .Pipe;
                    compile.stderr_behavior = .Pipe;
                    try compile.spawn();
                    _ = try compile.wait();
                }

                if (builtin.os.tag == .windows) {
                    const ps_cmd = try std.fmt.allocPrint(allocator, "& './a.out' | Tee-Object -FilePath '{s}'", .{tmp_output_path});
                    defer allocator.free(ps_cmd);

                    var run_process = std.process.Child.init(&[_][]const u8{ "pwsh", "-Command", ps_cmd }, allocator);
                    try run_process.spawn();
                    _ = try run_process.wait();
                } else {
                    // Use script command with explicit file
                    const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} ./a.out", .{tmp_output_path});
                    defer allocator.free(shell_cmd);

                    var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
                    try run_process.spawn();
                    _ = try run_process.wait();
                }

                // Read output file directly
                output = try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize));
            } else if (std.mem.eql(u8, extension, ".py")) {
                const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ dir_path, "/", entry.name });
                defer allocator.free(script_path);
                if (builtin.os.tag == .windows) {} else {
                    const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} python -u {s}", .{ tmp_output_path, script_path });
                    defer allocator.free(shell_cmd);

                    var shell_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
                    try shell_process.spawn();
                    _ = try shell_process.wait();
                }

                // Read output file directly
                output = try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize));
            }

            try entries.put(qal.items[index], FEntry{
                .extension = extension,
                .code = try allocator.dupe(u8, content),
                .index = index,
                .output = output,
            });
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
    const file = try std.fs.cwd().createFile("output.json", .{});
    defer file.close();
    try file.writeAll(json_array.items);
    try file.sync();
}
