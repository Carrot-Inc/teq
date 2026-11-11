// teq: --source 3.8
// A comprehension's trailing inline `map` whose receiver is a block: the statements lifted out of the call's prefix
// stand around the Inlined node (Inlines.inlineCall's liftBindings), which DropForMap.transformInlined transforms
// where it stands: the expansion gives way to the reference to the receiver's binding, the block's statements run.
class Box(val v: Int):
  inline def map(f: Int => Int): Box =
    println("map")
    new Box(f(this.v))

def make(): Box = new Box(1)

@main def test(): Unit =
  println((for x <- { println("receiver"); make() } yield x).v)
  println((for x <- { println("receiver"); make() } yield x + 1).v)
