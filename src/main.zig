const std = @import("std");
const builtin = @import("builtin");

const FEntry = struct {
    extension: []const u8,
    code: []const u8,
    index: usize,
    code_rtf: ?[]const u8 = null,
    output_rtf: []const u8,
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

    const questions_file = try std.fs.path.join(allocator, &[_][]const u8{ full_dir_path, "questions.txt" });
    defer allocator.free(questions_file);

    const raw_questions = std.fs.cwd().readFileAlloc(allocator, questions_file, std.math.maxInt(usize)) catch {
        std.debug.print("Failed to read questions file\n", .{});
        std.process.exit(1);
    };

    defer allocator.free(raw_questions);
    var questions_split = std.mem.split(u8, raw_questions, "\n---\n");

    // Parse questions with file mapping - use ArrayList to preserve order
    var question_file_map = std.StringHashMap([]const u8).init(allocator);
    defer question_file_map.deinit();
    var questions_list = std.ArrayList([]const u8).init(allocator);
    defer questions_list.deinit();
    var filename_list = std.ArrayList([]const u8).init(allocator);
    defer {
        for (filename_list.items) |filename| {
            allocator.free(filename);
        }
        filename_list.deinit();
    }

    while (questions_split.next()) |question_entry| {
        if (std.mem.indexOf(u8, question_entry, "::")) |sep_pos| {
            const question = std.mem.trim(u8, question_entry[0..sep_pos], " \n\r\t");
            const filename = std.mem.trim(u8, question_entry[sep_pos + 2 ..], " \n\r\t");
            try question_file_map.put(try allocator.dupe(u8, filename), try allocator.dupe(u8, question));
            try questions_list.append(try allocator.dupe(u8, question));
            try filename_list.append(try allocator.dupe(u8, filename));
        } else {
            // Fallback for old format - just add to questions list
            try questions_list.append(try allocator.dupe(u8, std.mem.trim(u8, question_entry, " \n\r\t")));
        }
    }

    var dir = std.fs.cwd().openDir(full_dir_path, .{}) catch {
        std.debug.print("Failed to open directory\n", .{});
        std.process.exit(1);
    };
    defer dir.close();

    var entries = std.ArrayList(struct { question: []const u8, entry: FEntry }).init(allocator);
    defer {
        for (entries.items) |item| {
            allocator.free(item.entry.extension);
            allocator.free(item.entry.code);
            allocator.free(item.entry.output_rtf);
            if (item.entry.code_rtf) |code_rtf| {
                allocator.free(code_rtf);
            }
        }
        entries.deinit();
    }

    const extension = try std.mem.concat(allocator, u8, &[_][]const u8{ ".", extension_arg });
    defer allocator.free(extension);

    var index: usize = 0;

    // Iterate through filename_list to preserve order
    for (filename_list.items) |filename| {
        if (!std.mem.endsWith(u8, filename, extension)) {
            continue;
        }

        const question = question_file_map.get(filename) orelse continue;

        // Check if file exists before trying to process it
        const file_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, filename });
        defer allocator.free(file_path);

        std.fs.cwd().access(file_path, .{}) catch {
            std.debug.print("Skipping missing file: {s}\n", .{filename});
            continue;
        };

        std.debug.print("\n=== Processing file: {s} ===\n", .{filename});

        // Rest of your processing logic remains the same...
        var file = dir.openFile(filename, .{}) catch {
            std.debug.print("Failed to open file: {s}\n", .{filename});
            continue;
        };

        const content = file.readToEndAlloc(allocator, std.math.maxInt(usize)) catch |err| {
            std.debug.print("Failed to read file: {s} ({})\n", .{ filename, err });
            file.close();
            continue;
        };
        file.close();

        var exec_command: []u8 = undefined;
        var exec_command_allocated = false;
        var classpath: []const u8 = "";

        // Prepare execution command based on extension
        if (std.mem.eql(u8, extension, ".cpp")) {
            const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, filename });
            defer allocator.free(script_path);

            const exe_name = try std.fmt.allocPrint(allocator, "{s}executable_{d}", .{ full_dir_path, index });
            defer allocator.free(exe_name);

            {
                var compile = std.process.Child.init(&[_][]const u8{ "g++", script_path, "-o", exe_name }, allocator);
                compile.stdout_behavior = .Pipe;
                compile.stderr_behavior = .Pipe;
                compile.spawn() catch {
                    std.debug.print("Failed to compile file: {s}\n", .{filename});
                    continue;
                };
                _ = try compile.wait();
            }

            exec_command = try allocator.dupe(u8, exe_name);
            exec_command_allocated = true;
        } else if (std.mem.eql(u8, extension, ".py")) {
            const script_path = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, filename });
            defer allocator.free(script_path);

            exec_command = try std.fmt.allocPrint(allocator, "python {s}", .{script_path});
            exec_command_allocated = true;
        } else if (std.mem.eql(u8, extension, ".java")) {
            const class_name = filename[0 .. filename.len - 5];
            classpath = full_dir_path[0 .. full_dir_path.len - 1];

            // Change working directory to the source folder
            var cwd_buffer: [std.fs.MAX_PATH_BYTES]u8 = undefined;
            const original_cwd = try std.process.getCwd(&cwd_buffer);
            const original_cwd_owned = try allocator.dupe(u8, original_cwd);
            defer allocator.free(original_cwd_owned);

            try std.process.changeCurDir(classpath);

            // Debug: List directory contents after changing CWD
            std.debug.print("Changed to directory: {s}\n", .{classpath});

            // Now exec_command can be simple since we're in the right directory
            exec_command = try std.fmt.allocPrint(allocator, "javac {s} && java {s}", .{ filename, class_name });
            exec_command_allocated = true;
            std.debug.print("Exec command: {s} (in directory: {s})\n", .{ exec_command, classpath });

            // We'll clean up the class file after execution and restore directory
            const class_file = try std.fmt.allocPrint(allocator, "{s}.class", .{class_name});
            defer {
                // Clean up class file
                std.fs.cwd().deleteFile(class_file) catch {};
                allocator.free(class_file);

                // Restore original working directory
                std.process.changeCurDir(original_cwd_owned) catch {};
            }
        }

        // Generate code RTF using pygmentize if color flag is set
        var code_rtf: ?[]const u8 = null;
        if (use_color) {
            const code_rtf_path = try std.fmt.allocPrint(allocator, "{s}output_rtf/code_{d}.rtf", .{ full_dir_path, index });
            defer allocator.free(code_rtf_path);

            // Create output_rtf folder if it doesn't exist
            const rtf_folder = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "output_rtf" });
            defer allocator.free(rtf_folder);
            std.fs.cwd().makeDir(rtf_folder) catch |err| {
                if (err != error.PathAlreadyExists) {
                    std.debug.print("Failed to create output_rtf folder\n", .{});
                }
            };

            const pygmentize_cmd = try std.fmt.allocPrint(allocator, "pygmentize -f rtf -O 'style=catppuccin-latte,fontface=CaskaydiaCove NF' {s}{s} > {s}", .{ full_dir_path, filename, code_rtf_path });
            defer allocator.free(pygmentize_cmd);

            var pygmentize_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", pygmentize_cmd }, allocator);
            pygmentize_process.spawn() catch {
                std.debug.print("Failed to generate code RTF for: {s}\n", .{filename});
            };
            _ = pygmentize_process.wait() catch {};

            // Read the generated RTF file
            const rtf_content = std.fs.cwd().readFileAlloc(allocator, code_rtf_path, std.math.maxInt(usize)) catch |err| {
                std.debug.print("Failed to read RTF file: {} for {s}\n", .{ err, filename });
                continue; // Skip this file if RTF reading fails
            };

            if (rtf_content.len > 0) {
                code_rtf = try allocator.dupe(u8, rtf_content);
                allocator.free(rtf_content);
            }
        }

        // Generate output RTF using termshot with --raw-write
        const output_rtf_path = try std.fmt.allocPrint(allocator, "{s}output_rtf/output_{d}.rtf", .{ full_dir_path, index });

        // Create output_rtf folder if it doesn't exist (in case color wasn't set above)
        const rtf_folder2 = try std.mem.concat(allocator, u8, &[_][]const u8{ full_dir_path, "output_rtf" });
        defer allocator.free(rtf_folder2);
        std.fs.cwd().makeDir(rtf_folder2) catch |err| {
            if (err != error.PathAlreadyExists) {
                std.debug.print("Failed to create output_rtf folder\n", .{});
            }
        };

        // Use termshot with --raw-write to capture RTF output
        // For Java files, we need to ensure termshot runs in the correct directory
        const termshot_cmd = if (std.mem.eql(u8, extension, ".java"))
            try std.fmt.allocPrint(allocator, "cd \"{s}\" && termshot --no-shadow --show-cmd --raw-write {s} -- \"{s}\"", .{ classpath, output_rtf_path, exec_command })
        else
            try std.fmt.allocPrint(allocator, "termshot --no-shadow --show-cmd --raw-write {s} -- {s}", .{ output_rtf_path, exec_command });
        defer allocator.free(termshot_cmd);

        var termshot_process = std.process.Child.init(&[_][]const u8{ "bash", "-c", termshot_cmd }, allocator);
        termshot_process.spawn() catch {
            std.debug.print("Failed to generate output RTF for: {s}\n", .{filename});
        };
        _ = termshot_process.wait() catch {
            std.debug.print("Failed to wait for termshot process\n", .{});
        };

        // Read the generated RTF output
        const output_rtf_content = std.fs.cwd().readFileAlloc(allocator, output_rtf_path, std.math.maxInt(usize)) catch |err| blk: {
            std.debug.print("Failed to read output RTF file: {} for {s}\n", .{ err, filename });
            break :blk try allocator.dupe(u8, "{\\rtf1 Failed to capture output}");
        };

        // Clean up the executable for C++ files
        if (std.mem.eql(u8, extension, ".cpp")) {
            const exe_to_delete = try std.fmt.allocPrint(allocator, "{s}executable_{d}", .{ full_dir_path, index });
            defer allocator.free(exe_to_delete);
            std.fs.cwd().deleteFile(exe_to_delete) catch {};
        }

        try entries.append(.{
            .question = question,
            .entry = FEntry{
                .extension = try allocator.dupe(u8, extension),
                .code = try allocator.dupe(u8, content),
                .index = index,
                .code_rtf = code_rtf,
                .output_rtf = output_rtf_content,
            },
        });

        // Clean up exec_command after use
        if (exec_command_allocated) {
            allocator.free(exec_command);
        }

        index += 1;
    }

    // Update JSON generation to include RTF fields instead of image paths
    var json_array = std.ArrayList(u8).init(allocator);
    defer json_array.deinit();

    const writer = json_array.writer();
    try writer.writeByte('[');

    var first = true;
    for (entries.items) |item| {
        if (!first) {
            try writer.writeByte(',');
        }
        first = false;

        try writer.writeByte('\n');
        try writer.writeAll("  {");
        try writer.writeAll("\n    \"question\": ");
        try std.json.stringify(item.question, .{}, writer);
        try writer.writeAll(",\n    \"index\": ");
        try std.json.stringify(item.entry.index, .{}, writer);
        try writer.writeAll(",\n    \"extension\": ");
        try std.json.stringify(item.entry.extension, .{}, writer);
        try writer.writeAll(",\n    \"code\": ");
        try std.json.stringify(item.entry.code, .{}, writer);
        try writer.writeAll(",\n    \"output_rtf\": ");
        try std.json.stringify(item.entry.output_rtf, .{}, writer);

        if (item.entry.code_rtf) |code_rtf_content| {
            try writer.writeAll(",\n    \"code_rtf\": ");
            try std.json.stringify(code_rtf_content, .{}, writer);
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

    // Run the Rust helper to generate Word document
    const rust_executable_path = if (builtin.mode == .Debug)
        "target/release/create-docx"
    else
        "/usr/local/bin/create-docx";

    var rust_process = std.process.Child.init(&[_][]const u8{ rust_executable_path, dir_path }, allocator);
    rust_process.stderr_behavior = .Pipe;
    rust_process.stdout_behavior = .Pipe;

    rust_process.spawn() catch |err| {
        std.debug.print("Failed to spawn Rust process: {}\n", .{err});
        return;
    };

    // Read stdout and stderr with null checks
    const stdout = if (rust_process.stdout) |stdout_pipe|
        stdout_pipe.reader().readAllAlloc(allocator, 10_000) catch ""
    else
        "";
    defer if (stdout.len > 0) allocator.free(stdout);

    const stderr = if (rust_process.stderr) |stderr_pipe|
        stderr_pipe.reader().readAllAlloc(allocator, 10_000) catch ""
    else
        "";
    defer if (stderr.len > 0) allocator.free(stderr);

    const exit_status = rust_process.wait() catch |err| {
        std.debug.print("Failed to wait for Rust process: {}\n", .{err});
        return;
    };

    switch (exit_status) {
        .Exited => |code| {
            if (code != 0) {
                std.debug.print("Rust process failed with exit code: {}\n", .{code});
                if (stderr.len > 0) {
                    std.debug.print("Error output: {s}\n", .{stderr});
                }
                return;
            }
        },
        .Signal => |signal| {
            std.debug.print("Rust process terminated by signal: {}\n", .{signal});
            if (stderr.len > 0) {
                std.debug.print("Error output: {s}\n", .{stderr});
            }
            return;
        },
        .Stopped => |signal| {
            std.debug.print("Rust process stopped by signal: {}\n", .{signal});
            if (stderr.len > 0) {
                std.debug.print("Error output: {s}\n", .{stderr});
            }
            return;
        },
        .Unknown => |status| {
            std.debug.print("Rust process exited with unknown status: {}\n", .{status});
            if (stderr.len > 0) {
                std.debug.print("Error output: {s}\n", .{stderr});
            }
            return;
        },
    }

    if (stdout.len > 0) {
        std.debug.print("Rust output: {s}\n", .{stdout});
    }

    std.debug.print("Word document generation completed!\n", .{});
}
