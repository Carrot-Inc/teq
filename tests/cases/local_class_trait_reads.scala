// A trait nested in a local class reads the block's values, as it reads the class's members,
// through its enclosing instance: the class holds them (dotty's `markFree`, `LambdaLift.proxyRef`).
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
