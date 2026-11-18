#!/usr/bin/env -S teq interp
!#
	val x = 1
  @main def main(): Unit = println(x)
// A script header that ends with a `!#` line is no part of the program, and the first token stands on the line
// after it: that line, one tab in, is the outermost region's width, and the next line, two spaces in, is neither
// wider nor narrower. scalac: "Incompatible combinations of tabs and spaces in indentation prefixes", the previous
// indent 1 tab, the latest 2 spaces. (The text comes after the program: a comment line first would put the first
// token on another line.)
// expect: 4:3: error: Incompatible combinations of tabs and spaces in indentation prefixes.
