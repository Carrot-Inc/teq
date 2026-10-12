package lca

// Local enums of a product's bodies with what their companions hold: a class case's default
// reading the block, an explicit companion's member reading the block, an object nested in it.
// The companion keeps its own module class (`Desugar.moduleDef`'s `E$`) apart from the enum,
// whoever wrote the products, and reads back whole (`TreePickler.pickleTree` keeps it all).
object Lib:
  def defaults(n: Int): Int =
    enum E:
      case A
      case B(x: Int = n)
    E.B() match
      case E.B(x) => x
      case _ => -1

  def explicit(n: Int): Int =
    enum E:
      case A
    object E:
      def get = n + A.ordinal
    E.get

  def helper(n: Int): Int =
    enum E:
      case A
    object E:
      object Helper:
        def get = n + 1
      def get = Helper.get + A.ordinal
    E.get
