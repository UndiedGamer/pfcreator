# Pfcreator

**Pfcreator** is a handy tool built with Rust and Zig that simplifies the creation of lab practical records. It automates the process of compiling code, running it, and neatly organizing the results into a `.docx` file. You can customize the look of your record using a straightforward `format.toml` configuration file.

> [!NOTE]  
> Pfcreator currently supports only `.cpp` and `.py`.

## What It Does

-   Automatically runs your `.cpp` or `.py` code files.
-   Creates a well-structured `.docx` practical record.
-   Lets you customize the formatting with a `format.toml` file.
-   Uses a `questions.txt` file to organize your practical questions.

## How to Use

### 1. Installation

*Will provide binaries later*

### 2. Set Up Your Files

1. **Code Files:** Put all your `.cpp` or `.py` files into a single folder.
> [!IMPORTANT]  
> Pfcreator works with either `.cpp` (C++) or `.py` (Python) files at a time, not both simultaneously.

2. **`questions.txt`:**  In the same folder, create a file named `questions.txt`. Write each practical question, followed by a line of `---` to separate them. Like this:

    ```
    Question 1
    ---
    Question 2
    ---
    Question 3
    ```

3. **`format.toml`:** Also in the same folder, create a `format.toml` file to define how your record should look. Here's an example:

    ```toml
    [header]
    size = 14             # Font size (optional, default is 12)
    bold = true           # Bold text (optional, default is false)
    text = "Task {n}"     # {n} will be the question number
    align = "center"      # Text alignment (optional, default is left)

    [question]
    size = 17
    bold = true
    text = "Q) {question}" # {question} will be taken from questions.txt

    [solution]
    size = 12
    text = "{solution}"   # {solution} will be your code

      [solution.title]
      size = 14
      bold = true
      underline = true
      text = "Code:"

    [output]
    size = 12
    text = "{output}"    # {output} will be the program's output

      [output.title]
      size = 14
      bold = true
      underline = true
      text = "Output:"

    # Optional footer
    [footer]
    size = 10
    text = "Made by Hemanth"
    align = "right"
    ```

### 3. Create Your Record

1. Open your terminal and go to the folder with your files.
2. Run this command:

    ```bash
    pfcreator <file_extension> <folder_path>
    ```

    -   Replace `<file_extension>` with `cpp` or `py` (depending on your code files).
    -   Replace `<folder_path>` with the path to your folder.

    **For example:**

    ```bash
    pfcreator cpp my_cpp_practical
    ```

    or

    ```bash
    pfcreator py my_python_practical
    ```

Your `.docx` practical record will be created inside the folder you specified (`my_cpp_practical` or `my_python_practical` in the examples).

## Placeholders in `format.toml`

These are special tags in your `format.toml` that Pfcreator replaces:

-   `{n}`: The question number.
-   `{question}`: The question from `questions.txt`.
-   `{solution}`: Your code.
-   `{output}`: The output of your code.

Note to self: add windows deprecation in future section here