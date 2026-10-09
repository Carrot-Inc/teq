package sharedlib

object Traits:
  trait HasHelper:
    def helper(x: Int): Int = x + 1

trait PreludeCore:
  export sharedlib.Traits.*
