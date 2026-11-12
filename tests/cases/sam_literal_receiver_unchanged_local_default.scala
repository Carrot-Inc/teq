// Where the receiver of sam_literal_receiver_shapes is not taken: a local class's constructor default, whose
// getter is no body of the method around it. The unqualified `synchronized` keeps the receiver it had.
trait Job { def go(): String }
class O(val tag: String):
  def local(): String =
    class L(val j: Job = () => synchronized { tag })
    new L().j.go()
@main def run(): Unit = println(new O("o").local())
