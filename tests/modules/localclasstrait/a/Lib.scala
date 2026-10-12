package lct

// A trait a class of a block owns reads the block's values through the enclosing instances, from
// its members and its nested classes at any depth (dotty's `markFree`, `LambdaLift.proxyRef`):
// the pickle keeps the reads as written, and a module over the products reads them back.
object Lib:
  def deep(n: Int): Int =
    class O:
      trait T:
        class A:
          class B:
            def get = n
          def get = (new B).get
        def get = (new A).get
      def get = (new T {}).get
    (new O).get

  def counter(start: Int): String =
    var count = start
    class O:
      trait T:
        def bump(by: Int): Unit = count += by
        class A:
          def twice(): Unit = { bump(1); count = count * 2 }
        def run(): Int = { (new A).twice(); count }
      def go = (new T {}).run()
    val r = (new O).go
    count += 100
    s"$r $count"

  def calls(n: Int): Int =
    def helper(x: Int) = x + n
    lazy val once = n * 10
    class O:
      trait T:
        def viaDef = helper(1)
        def viaLazy = once + once
        class A:
          def both = helper(2) + once
      def get = { val t = new T {}; t.viaDef + t.viaLazy + (new t.A).both }
    (new O).get

class Host(val base: Int):
  def run(n: Int): Int =
    class O:
      trait T:
        def get = base + n + Host.this.base
      def get = (new T {}).get
    (new O).get
