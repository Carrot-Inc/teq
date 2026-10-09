// A block a partial function's lambda ends in a match: dotty's `typedBlock` types it, and where
// its type names a definition of the block (`escapingRefs`) `ensureNoLocalRefs` ascribes it a
// type that avoids them, so that it ends in no match (no E211, the match checked as any);
// otherwise its match is not the partial function (E211). The type is dotty's: a call's result
// (`O.n()` an `Int`), a reference's singleton (`k`), the join of the branches, which a wider
// branch absorbs `k` into, a construction's class; the traversal reaches every part of it, a
// type argument (`List[k.type]`), a tuple's element or a map's value.
object Witness:
  val leaks: PartialFunction[Boolean, Int] = b => { object O { def n(): Int = 1 }; b match { case true => O.n() } }
  val joined: PartialFunction[Boolean, Any] = b => { val k = 1; b match { case true => (2: Int); case false => k } }
  val local: PartialFunction[Boolean, Any] = b => { val k = 1; b match { case true => k } }
  val both: PartialFunction[Boolean, Any] = b => { val k = 1; val j = 2; b match { case true => k; case false => j } }
  val cls: PartialFunction[Boolean, Any] = b => { class C; b match { case true => new C } }
  val listed: PartialFunction[Boolean, Any] = b => { class C; b match { case true => List(new C) } }
  val obj: PartialFunction[Boolean, Any] = b => { object O; b match { case true => O } }
  val member: PartialFunction[Boolean, Any] = b => { object O { val v = 1 }; b match { case true => O.v } }
  val defined: PartialFunction[Boolean, Any] = b => { object O { def m: Int = 1 }; b match { case true => O.m } }
  val ascribed: PartialFunction[Boolean, Any] = b => { val k = 1; b match { case true => (k: Int) } }
  val argument: PartialFunction[Boolean, Any] = b => { val k = 1; b match { case true => List[k.type](k); case false => List[k.type](k) } }
  val partly: PartialFunction[Boolean, Any] = b => { val k = 1; b match { case true => List[k.type](k) } }
  val element: PartialFunction[Boolean, Any] = b => { val k = "s"; b match { case true => Some[(Int, k.type)]((1, k)); case false => None } }
  val valued: PartialFunction[Boolean, Any] = b => { object O { val v = 1 }; b match { case true => Map[Int, O.type](1 -> O) } }

@main def run(): Unit =
  println(List(Witness.leaks.isDefinedAt(false), Witness.joined.isDefinedAt(false), Witness.local.isDefinedAt(false)))
