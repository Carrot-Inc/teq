package lcb

// Local classes, traits and objects that nothing instantiates: their bodies depend on the
// upstream all the same.
def f: Int =
  class Local:
    def g: Int = lca.Lib.n
  0

class Host:
  def h: Int =
    trait T:
      def t: String = lca.Lib.s
    object O:
      val o: Int = lca.Lib.n
    1
