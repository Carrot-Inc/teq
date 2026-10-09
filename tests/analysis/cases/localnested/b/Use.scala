package lnb

// Classes nested in local classes, which nothing instantiates either: their bodies depend on the
// upstream all the same.
def f: Int =
  class Local:
    class Nested:
      def g: Int = lna.Lib.n
  0

class Host:
  def h: Int =
    class Outer:
      object Inner:
        val o: String = lna.Lib.s
    1
