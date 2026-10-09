#!/usr/bin/env -S teq interp
// A script's first line, `#!` and what runs it, is skipped as scalac skips it (scalac's
// `ScriptSourceFile`); tests/errors/script_header.scala checks that the lines after it keep their
// numbers.
object Main:
  def main(args: Array[String]): Unit =
    println("the header is skipped")
