// `C.this` names the enclosing class `C`, and a `self =>` alias is that class's `this`
// wherever it is in scope: in nested anonymous classes, their lambdas and their types.
trait Sched[-In, +Out]:
  self =>
  type State
  def initial: State
  def step(in: In, state: State): (State, Out)
  def &&[In1 <: In, Out2](that: Sched[In1, Out2]): Sched[In1, (Out, Out2)] =
    new Sched[In1, (Out, Out2)]:
      override type State = (self.State, that.State)
      val initial: State = (self.initial, that.initial)
      def step(in: In1, state: State): (State, (Out, Out2)) =
        val (l, o1) = self.step(in, state._1)
        val (r, o2) = that.step(in, state._2)
        ((l, r), (o1, o2))

object Count extends Sched[Any, Int]:
  type State = Int
  def initial = 0
  def step(in: Any, state: Int) = (state + 1, state)

def run[In, Out](s: Sched[In, Out], in: In, n: Int): List[Out] =
  var st = s.initial
  (1 to n).toList.map { _ => val (st2, o) = s.step(in, st); st = st2; o }

trait Named { def name: String }

trait Braced[A] { outer =>
  def name: String
  def child: Named = new Named {
    def name = "child of " + outer.name + " and " + Braced.this.name
    val same: outer.type = outer
    def names = List(1, 2).map(i => outer.name + i)
    override def toString = names.mkString(",") + (same eq outer)
  }
}

class Box(val v: Int):
  def name = "box" + v
  def wrap: Box = new Box(v + 1):
    override def name = "in " + Box.this.name + " " + this.v
  def deep: Named = new Named:
    self =>
    def name = "deep"
    def inner: Named = new Named:
      def name = self.name + " of " + Box.this.name
    override def toString = inner.name

object Registry:
  reg =>
  val prefix = "reg"
  def make: Named = new Named:
    def name = reg.prefix + "/" + Registry.this.prefix

@main def Main(): Unit =
  println(run(Count && Count, (), 3))
  object X extends Braced[Int] { def name = "x" }
  println(X.child.name)
  println(X.child)
  println(Box(1).wrap.name)
  println(Box(2).deep)
  println(Registry.make.name)
