// A `return` in a local lazy val's initialiser, whose value is the def's result too: a return that left the
// initialiser instead of the def would print the same, so the program that tells the two apart is
// lazy_val_return_leaves_the_def.
def f(): Int =
  lazy val x: Int = return 7
  x
@main def run(): Unit = println(f())
