const std = @import("std");
const builtin = @import("builtin");

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
    code_image: ?[]const u8 = null,
    output_image: []const u8,
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

    // Check for color flag
    var use_color = false;
    var extension_arg: []const u8 = "";
    var dir_path: []const u8 = "";
    
    while (args_it.next()) |arg| {
        if (std.mem.eql(u8, arg, "color")) {
            use_color = true;
        } else if (extension_arg.len == 0) {
            extension_arg = arg;
        } else if (dir_path.len == 0) {
            dir_path = arg;
            break;
        }
    }

    if (extension_arg.len == 0 or dir_path.len == 0) {
        std.debug.print("Usage: <program> [color] <extension> <folder>\n", .{});
        std.process.exit(1);
    }

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
    
    // Create output folder for images
    const output_folder = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "output_images" });
    defer allocator.free(output_folder);
    std.fs.cwd().makeDir(output_folder) catch |err| {
        if (err != error.PathAlreadyExists) {
            std.debug.print("Failed to create output_images folder\n", .{});
            std.process.exit(1);
        }
    };

    const questions_file = try std.fs.path.join(allocator, &[_][]const u8{ full_dir_path, "questions.txt" });
    defer allocator.free(questions_file);

    const raw_questions = std.fs.cwd().readFileAlloc(allocator, questions_file, std.math.maxInt(usize)) catch {
        std.debug.print("Failed to read questions file\n", .{});
        std.process.exit(1);
    };

    defer allocator.free(raw_questions);
    var questions_split = std.mem.split(u8, raw_questions, "\n---\n");
    
    // Parse questions with file mapping
    var question_file_map = std.StringHashMap([]const u8).init(allocator);
    defer question_file_map.deinit();
    var questions_list = std.ArrayList([]const u8).init(allocator);
    defer questions_list.deinit();

    while (questions_split.next()) |question_entry| {
        if (std.mem.indexOf(u8, question_entry, "::")) |sep_pos| {
            const question = std.mem.trim(u8, question_entry[0..sep_pos], " \n\r\t");
            const filename = std.mem.trim(u8, question_entry[sep_pos + 2..], " \n\r\t");
            try question_file_map.put(filename, question);
            try questions_list.append(question);
        } else {
            // Fallback for old format - just add to questions list
            try questions_list.append(std.mem.trim(u8, question_entry, " \n\r\t"));
        }
    }

    var dir = std.fs.cwd().openDir(full_dir_path, .{}) catch {
        std.debug.print("Failed to open directory\n", .{});
        std.process.exit(1);
    };
    defer dir.close();

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
    var file_iter = question_file_map.iterator();
    while (file_iter.next()) |entry| {
        const filename = entry.key_ptr.*;
        const question = entry.value_ptr.*;
        
        if (!std.mem.endsWith(u8, filename, extension)) {
            continue;
        }

        std.debug.print("\n=== Processing file: {s} ===\n", .{filename});
        var file = dir.openFile(filename, .{}) catch {
            std.debug.print("Failed to open file: {s}\n", .{filename});
            continue;
        };
        const content = file.readToEndAlloc(allocator, std.math.maxInt(usize)) catch {
            std.debug.print("Failed to read file: {s}\n", .{filename});
            file.close();
            continue;
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
        var code_image: ?[]const u8 = null;
        var exec_command: []const u8 = "";

        // Execute file based on extension
        if (std.mem.eql(u8, extension, ".cpp")) {
            const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "/", filename });
            defer allocator.free(script_path);

            {
                var compile = std.process.Child.init(&[_][]const u8{ "g++", script_path, "-o", "./a.out" }, allocator);
                compile.stdout_behavior = .Pipe;
                compile.stderr_behavior = .Pipe;
                compile.spawn() catch {
                    std.debug.print("Failed to compile file: {s}\n", .{filename});
                    continue;
                };
                _ = try compile.wait();
            }
            defer std.fs.cwd().deleteFile("./a.out") catch {};

            exec_command = "./a.out";
            const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} ./a.out", .{tmp_output_path});
            defer allocator.free(shell_cmd);

            var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
            run_process.spawn() catch {
                std.debug.print("Failed to run file: {s}\n", .{filename});
                continue;
            };
            _ = try run_process.wait();

            output = try processOutput(allocator, try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize)));
        } else if (std.mem.eql(u8, extension, ".py")) {
            const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "/", filename });
            defer allocator.free(script_path);
            
            exec_command = try std.fmt.allocPrint(allocator, "python {s}", .{filename});
            defer allocator.free(exec_command);
            
            const shell_cmd = try std.fmt.allocPrint(allocator, "script -q {s} python -u {s}", .{ tmp_output_path, script_path });
            defer allocator.free(shell_cmd);

            var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
            run_process.spawn() catch {
                std.debug.print("Failed to run file: {s}\n", .{filename});
                continue;
            };
            _ = try run_process.wait();

            output = try processOutput(allocator, try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize)));
        } else if (std.mem.eql(u8, extension, ".java")) {
            const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "/", filename });
            defer allocator.free(script_path);

            const class_name = filename[0 .. filename.len - 5];
            {
                var compile = std.process.Child.init(&[_][]const u8{ "javac", script_path }, allocator);
                compile.stdout_behavior = .Pipe;
                compile.stderr_behavior = .Pipe;
                compile.spawn() catch {
                    std.debug.print("Failed to compile file: {s}\n", .{filename});
                    continue;
                };
                _ = try compile.wait();
            }
            const class_file = try std.fmt.allocPrint(allocator, "{s}/{s}.class", .{ full_dir_path, class_name });
            defer allocator.free(class_file);
            defer std.fs.cwd().deleteFile(class_file) catch {};

            exec_command = try std.fmt.allocPrint(allocator, "java {s}", .{class_name});
            defer allocator.free(exec_command);

            const shell_cmd = try std.fmt.allocPrint(allocator, "cd {s} && script -q {s} java {s}", .{ full_dir_path, tmp_output_path, class_name });
            defer allocator.free(shell_cmd);

            var run_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", shell_cmd }, allocator);
            run_process.spawn() catch {
                std.debug.print("Failed to run file: {s}\n", .{filename});
                continue;
            };
            _ = try run_process.wait();

            output = try processOutput(allocator, try std.fs.cwd().readFileAlloc(allocator, tmp_output_path, std.math.maxInt(usize)));
        }

        // Generate code image using pygmentize if color flag is set
        if (use_color) {
            const code_image_path = try std.fmt.allocPrint(allocator, "{s}output_images/code_{d}.rtf", .{ full_dir_path, index });
            defer allocator.free(code_image_path);
            
            const pygmentize_cmd = try std.fmt.allocPrint(allocator, "cd {s} && pygmentize -f rtf -O style=catppuccin-latte,fontface=\"CaskaydiaCove NF\",fontsize=11 {s} > {s}", .{ full_dir_path, filename, code_image_path });
            defer allocator.free(pygmentize_cmd);

            var pygmentize_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", pygmentize_cmd }, allocator);
            pygmentize_process.spawn() catch {
                std.debug.print("Failed to generate code image for: {s}\n", .{filename});
            };
            _ = pygmentize_process.wait() catch {};

            code_image = try allocator.dupe(u8, code_image_path);
        }

        // Always generate output image using termshot
        const output_image_path = try std.fmt.allocPrint(allocator, "{s}output_images/output_{d}.png", .{ full_dir_path, index });
        defer allocator.free(output_image_path);
        
        const termshot_cmd = try std.fmt.allocPrint(allocator, "cd {s} && termshot -- \"{s}\"", .{ full_dir_path, exec_command });
        defer allocator.free(termshot_cmd);

        var termshot_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", termshot_cmd }, allocator);
        termshot_process.stdout_behavior = .Pipe;
        
        termshot_process.spawn() catch {
            std.debug.print("Failed to generate output image for: {s}\n", .{filename});
            continue;
        };
        
        const termshot_result = termshot_process.wait() catch {
            std.debug.print("Failed to wait for termshot process\n", .{});
            continue;
        };

        // Read termshot output and save to file
        if (termshot_result == .Exited and termshot_result.Exited == 0) {
            const termshot_output = try termshot_process.stdout.?.reader().readAllAlloc(allocator, std.math.maxInt(usize));
            defer allocator.free(termshot_output);
            
            const output_file = try std.fs.cwd().createFile(output_image_path, .{});
            defer output_file.close();
            try output_file.writeAll(termshot_output);
        }

        const final_output_image_path = try allocator.dupe(u8, output_image_path);

        try entries.put(question, FEntry{
            .extension = extension,
            .code = try allocator.dupe(u8, content),
            .index = index,
            .output = output,
            .code_image = code_image,
            .output_image = final_output_image_path,
        });
        index += 1;
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
        try writer.writeAll(",\n    \"output_image\": ");
        try std.json.stringify(entry.value_ptr.output_image, .{}, writer);
        
        if (entry.value_ptr.code_image) |code_img| {
            try writer.writeAll(",\n    \"code_image\": ");
            try std.json.stringify(code_img, .{}, writer);
        }
        
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
