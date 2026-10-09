#!/bin/sh
exec teq interp "$0" "$@"
!#
// A header of several lines runs through its first line that starts with `!#`, as scalac reads
// one (`ScriptSourceFile`): the two lines above it are no code.
object Main:
  def main(args: Array[String]): Unit =
    println("the header of three lines is skipped")
