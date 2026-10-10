package edits.blank_lines
object O { val x = 1; val y = 2 }
object P { val q = 1 }

import O.x

import P.q
object Main {

  import O.y

  def run: Int = 1
}
