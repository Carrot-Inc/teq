package lca

object Holder:
  def make: Int =
    class Loc:
      def v: Int = 2
    new Loc().v

class Sibling:
  def s: Int = 1
