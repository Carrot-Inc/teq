// jars: scala-library
// scala-library's `Tuple.head[This >: this.type <: NonEmptyTuple]: Head[This]` on a cons over
// type parameters: the bound `this.type` is seen from the receiver, so `This` is the receiver's
// type and `Head[A *: B]` reduces to `A`.
def head[A, B <: Tuple](t: A *: B): A = t.head
def tail[A, B <: Tuple](t: A *: B): B = t.tail
def second[A, B, C <: Tuple](t: A *: B *: C): B = t.tail.head
