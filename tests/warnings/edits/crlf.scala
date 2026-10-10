package edits.crlf
object O { val x = 1; val y = 2; val z = 3 }

import O.x

import O.{y, z}
object Main { def run: Int = z }
