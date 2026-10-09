package lca

object Holder:
  def make: Int =
    class Loc:
      def v: Int = 1
    new Loc().v

class Sibling:
  def s: Int = 1
