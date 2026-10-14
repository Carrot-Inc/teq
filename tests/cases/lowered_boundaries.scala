// The boundaries between the interpreter's lowered code and its walker with captured mutable
// state (src/interp/code.rs: a body's first call is walked, later ones run lowered): a lowered
// caller handing closures over its `var` to a callee on its first call, a caller on its first call
// handing its own to a lowered callee, and a local trait's member reached both with its proxies
// passed and through `super[T]` with them left in the caller's frame, two incoming layouts of one
// unit, each mutating a local of the block that declares the trait.
object Main:
  def runner(i: Int, f: (() => Int, Int => Unit) => Int): Int =
    var x = i
    val y = f(() => x, d => x += d)
    x * 100 + y

  def walk1(get: () => Int, add: Int => Unit): Int = { add(5); get() }
  def walk2(get: () => Int, add: Int => Unit): Int = { add(7); add(1); get() * 2 }
  def walk3(get: () => Int, add: Int => Unit): Int = { add(get()); get() + 3 }

  def bump(get: () => Int, add: Int => Unit): Int = { add(3); get() + 1 }

  def caller1(): Int = { var z = 1; val r = bump(() => z, d => z += d); z * 10 + r }
  def caller2(): Int = { var z = 2; val r = bump(() => z, d => z *= d); z * 10 + r }
  def caller3(): Int = { var z = 3; val r = bump(() => z, d => z -= d); z * 10 + r }

  def traits(n: Int): String =
    var count = n
    trait Counter:
      def next(): Int = { count += 1; count }
    class Twice extends Counter:
      override def next(): Int = super[Counter].next() * 10 + super[Counter].next()
    val plain = new Counter {}
    val twice = new Twice
    s"${plain.next()} ${twice.next()} ${plain.next()} $count"

  def main(args: Array[String]): Unit =
    println(runner(1, walk1))
    println(runner(2, walk2))
    println(runner(3, walk3))
    println(bump(() => 0, _ => ()))
    println(bump(() => 1, _ => ()))
    println(caller1())
    println(caller2())
    println(caller3())
    for i <- 1 to 3 do println(traits(i))
