// expect: 9:11: error: not supported yet: a trait nested in a local class referring to n of its enclosing block; move the class to the top level
// expect: 1 error found

// A trait has no constructor to take the block's values (scalac's `LambdaLift` gives its methods
// proxies of them): one nested in a local class that reads them is refused, not left to read a
// name its output does not have. One that reads the class's members is fine.
def f(n: Int): Int =
  class C:
    trait T:
      def get = n
    def get = (new T {}).get
  (new C).get
def g(n: Int): Int =
  class D:
    val m = n
    trait U:
      def get = m
    def get = (new U {}).get
  (new D).get
@main def run(): Unit = println(f(7) + g(8))
