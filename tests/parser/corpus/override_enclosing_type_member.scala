// An anonymous subclass's `State` names the enclosing instance's `Sched.this.State`: its members
// override the parent's as seen from the subclass, and its own members keep the enclosing `this`.
trait Sched[-In, +Out]:
  type State
  def initial: State
  def step(in: In, state: State): (State, Out)
  def both[In1 <: In, Out2](that: Sched[In1, Out2]): Sched[In1, (Out, Out2)] =
    val me: Sched[In, Out] { type State = Sched.this.State } = this
    new Sched[In1, (Out, Out2)]:
      type State = (Sched.this.State, that.State)
      val initial: State = (me.initial, that.initial)
      private def left(in: In1, l: Sched.this.State): (Sched.this.State, Out) = me.step(in, l)
      def step(in: In1, state: State): (State, (Out, Out2)) =
        val (l, o1) = left(in, state._1)
        val (r, o2) = that.step(in, state._2)
        ((l, r), (o1, o2))

object Count extends Sched[Any, Int]:
  type State = Int
  def initial = 0
  def step(in: Any, state: Int) = (state + 1, state)

object Twice extends Sched[Any, String]:
  type State = String
  def initial = ""
  def step(in: Any, state: String) = (state + "a", state)

def run[In, Out](s: Sched[In, Out], in: In, n: Int): List[Out] =
  var st = s.initial
  (1 to n).toList.map { _ => val (st2, o) = s.step(in, st); st = st2; o }

@main def Main(): Unit =
  println(run(Count.both(Twice), (), 3))
  println(run(Count.both(Count).both(Twice), (), 2))
