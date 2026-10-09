// A stored inline body's record is renumbered by the parallel typer's merge with the types in it:
// `y.type`, a type argument naming a local of the body, becomes the merged local's
// (`TEQ_FORK=1 TEQ_MERGE_CHECK=1` fails the build on a stored record that still names a worker's
// id).
def id[A](a: A): A = a
inline def f(x: AnyRef) = { val y = x; id[y.type](y) }
@main def run(): Unit = println(f("s"))
