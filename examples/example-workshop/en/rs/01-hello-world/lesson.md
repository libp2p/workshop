Welcome to your first Rust lesson! In this lesson, you'll write a simple program that prints "Hello, World!" to the console.

## Introduction

"Hello, World!" is traditionally the first program you write when learning a new programming language. It's a simple way to make sure that your development environment is set up correctly and to learn the basic syntax of a language.

In Rust, a "Hello, World!" program demonstrates:
- The main function, which is the entry point of every Rust program
- How to print text to the console
- The basic structure of a Rust project

## Your Task

Create a Rust program that prints "Hello, World!" to the console.

To complete this task:

1. Edit the file named `main.rs` in the `src/` subfolder of your Rust project
2. Write a Rust program that prints "Hello, World!" to the console
3. Run the check command to verify your solution

## Hint - Getting Started

The `main.rs` file is created automatically in the `src/` subfolder when you create a new Rust project using Cargo. Open it with your favorite text editor or IDE.

Here's the basic structure of a Rust program:

```rust
fn main() {
    // Your code goes here
}
```

The `main` function is the entry point of your program. When you run your program, execution begins at the first line of the `main` function.

## Hint - Printing to the Console

In Rust, you can print text to the console using the `println!` macro. A macro is a special kind of function that is expanded at compile time.

The syntax for printing a string is:

```rust
println!("Your text here");
```

Notice the exclamation mark after `println`. This indicates that it's a macro, not a regular function.

## Hint - Complete Solution

Here's the complete solution:

```rust
fn main() {
    println!("Hello, World!");
}
```

To run this program, you would typically use the `cargo run` command, but for this lesson, the check command will compile and run your code for you.

## Conclusion

Congratulations on writing your first Rust program! This simple example demonstrates the basic structure of a Rust program and how to print text to the console.

As you progress through more lessons, you'll learn about variables, types, functions, and other Rust features. But every Rust program you write will start with a `main` function, just like this one.
