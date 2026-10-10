package npp

// Parents through prefixes in an upstream module: a trait of an instance's member
// (`extends mid.T`, whose products write the prefixed parent) and a class of an object's
// value (`extends H.o.I`, whose constructor takes `H.o` as its enclosing instance).
class O(val n: Int):
  class Mid:
    trait T:
      def get: Int = n
  val mid = new Mid
  class C extends mid.T
  class I:
    def value = n

object H:
  val o = new O(7)

class D extends H.o.I
