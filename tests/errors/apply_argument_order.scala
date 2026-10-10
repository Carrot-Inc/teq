// expect: 6:40: error: value + is not a member of Any
// An application's arguments are typed in the parameters' order, a function literal among them
// (dotty's `Applications.matchArgs`): the literal's parameter takes its type from the formal as it
// stands then, `A` not yet bounded by the `1` after it, and is an `Any`, as under scalac.
def f[A](g: A => A, x: A): A = g(x)
@main def run(): Unit = println(f(x => x + 1, 1))
