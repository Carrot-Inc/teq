// jars: stubparent-lib
// A member's completion completes every class its signature names, a class the class path holds too (`cook` asks
// `Present` for its type parameters, `dependencyStatus` reads its information), and that class's own completion cooks
// its parents: `Present`'s parent `Gone`, which the jar does not hold, is the stub scalac 3.8.4 reports.
// expect: error: Bad symbolic reference. A signature
// expect: refers to Gone/T in package stubp which is not available.
@main def run(): Unit =
  val x: Any = stubp.Ref.get()
  println("done")
