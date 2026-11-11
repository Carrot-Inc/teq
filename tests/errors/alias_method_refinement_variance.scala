// An alias whose right side refines by a method takes its parameter's sign from the method's info: a parameter's type
// at the flipped variance, the result's at the variance (`TypeAccumulator.foldOver`'s `LambdaType` arm, Types.scala
// 6972-6978). So `Sink` is contravariant and `Source` covariant. scalac 3.8.4: two errors, the sink widened at line
// 13 and the source narrowed at line 14.
// expect: 13:35: error: Found:    (s : Sink[B])
// expect: 14:43: error: Found:    (s : Source[A])
// expect: 2 errors found
trait A
trait B extends A
type Sink[X] = AnyRef { def put(xs: List[X]): Unit }
type Source[X] = AnyRef { def get(): List[X] }

def sinkUp(s: Sink[B]): Sink[A] = s
def sourceDown(s: Source[A]): Source[B] = s
