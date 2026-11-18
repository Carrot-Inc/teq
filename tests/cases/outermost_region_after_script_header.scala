#!/usr/bin/env -S teq interp
!#
    def f: String =
    "one"
    @main def main(): Unit = println(f)
// A script header that ends with a `!#` line is no part of the program, and the first token stands on the line
// after it: that line, four spaces in, is the outermost region's width, so the right side on the line after `=`,
// at that width, is no indented block, and the line after it is a statement of its own. (The text comes after the
// program: a comment line first would put the first token on another line.) scalac prints one.
