#!/usr/bin/env -S teq interp
// The lines after a script header keep their numbers: the error is on line 6.
// expect: 6:16: error: not found: nope
// expect: 1 error found
object Main:
  val x: Int = nope
