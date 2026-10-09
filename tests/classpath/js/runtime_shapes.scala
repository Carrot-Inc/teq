// jars: fixtures
// targets: js interp
// Not on the JVM, where link mode calls a default getter of the fixture's class file with the
// parameter before it, `Refs.Named.apply$default$2(String)`, which scalac's takes none
// of.
// Bodies of a jar with the shapes zio's runtime has: a method-local sealed trait with case
// objects and a case class below it (`Cause.foldContext`), local type aliases (`FiberRefs`),
// a var left to its default value (`ChannelExecutor`).
import fix.runtime.{Bag, Cause, Cell, Folds, Metric, PatchRef, Promise, Ref, Slot, Slots, Tokens, Concurrents, Pairs, Wilds, Atomics, Refs, Inspectors, Apps, Clocks}

@main def main(): Unit =
  println(Folds.fold(List(3, 0, 1, -7, -2, 9)))
  println(Folds.counters(3))
  println(Folds.pick(List(1)))
  println(Folds.pairs(3))
  val c = new Cell
  println(c.show)
  c.set("a")
  println(c.show)
  val m = Metric.counter()
  m.unsafe.update(3L)
  println(m.unsafe.value)
  println(Metric.track(Metric.lengths(), "hello"))
  val p = new Promise[String, Int]
  println(Promise.succeed(p.asInstanceOf[Promise[Nothing, Int]], 4))
  println(p.unsafe.completeWith(Left("late")))
  Promise.run(p, r => println(s"observed $r"))
  println(p.poll)
  println(PatchRef("x").get)
  val r: Ref[Int] = PatchRef(1)
  println(r.initial + 1)
  println(Ref.unsafe.make("y").get)
  println(Ref.unsafe.SelfRef(2).twice)
  println(Cause.Filter[Int](_ => true).failCase(1))
  println(Cause.Filter[Int](_ => false).failCase(1))
  val bag = Bag(1, 2, 2)
  val more: Bag[Int] = bag :+ 3
  val once: Bag[Int] = bag.distinct
  println(more)
  println(once)
  println(bag.plus(4).unique)
  println(Slots.diff(Map(PatchRef(1) -> Slots(::(Slot(2), Nil), 1), PatchRef("a") -> Slots(::(Slot("a"), Nil), 1))).length)
  println(Tokens.run)
  println(Tokens.sameClass(new fix.runtime.Token(1)))
  println(Concurrents.ints.hold(1, "x").held)
  val pr = Pairs.ints.pair("b")
  println(pr.show + " " + (pr.a + 1))
  println(Wilds.union(Set(Wilds.Leaf("b"), Wilds.Leaf("a"))))
  println(Wilds.renderDb(Map(Wilds.Leaf("d") -> Set(Wilds.Leaf("c"), Wilds.Leaf("a")), Wilds.Leaf("b") -> Set.empty[Wilds.Leaf])))
  println(Wilds.firsts(Wilds.Holder[Any](List(Wilds.Cell(1), Wilds.Cell("x")))))
  given Atomics.Token = new Atomics.Token
  val atomic = Atomics.make(1)
  atomic.set(5)
  println(atomic.get)
  println(atomic.updateAndGet(_ + 1))
  val named = Refs.Named("a")
  println(named)
  println(named.copy(prefix = Some(Refs.Named("p"))).prefix)
  println(Wilds.names(Iterator(Wilds.Leaf("x"), Wilds.Leaf("y"))))
  println(Inspectors.run())
  println(Refs.single("k"))
  println(ShapesApp.run)
  println(Clocks.Clock.Live.unsafe.now())

object ShapesApp extends Apps.DefaultApp
