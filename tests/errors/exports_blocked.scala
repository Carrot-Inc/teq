// expect: type MissingTrait not found
// expect: cannot resolve export: MissingObj not found
// expect: cannot resolve export: Other not found
// expect: cannot export nope: it is not a member of Inner

package util

import util.Prelude.*

object Traits:
  trait HasHelper:
    def helper(x: Int): Int = x + 1

trait PreludeCore:
  export Traits.*

// The lookups for the parents and the export paths of `Utils` go through the prelude, which
// exports `Utils` in turn. What stays unresolved is still reported.
object Utils extends HasHelper, MissingTrait:
  export MissingObj.*
  export Other.{a, b}

  object Inner:
    def fine: Int = 1

object Prelude extends PreludeCore:
  export Utils.*
  export Inner.{fine, nope}

@main def run(): Unit =
  println(fine)
