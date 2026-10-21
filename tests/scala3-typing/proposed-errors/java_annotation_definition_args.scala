// A definition's Java annotation typed as `Applications.reorder` and `matchArgs` type its constructor's arguments, a
// binder's alike: scalac 3.8.4's two errors. teq types a definition's annotations quietly (master's), a part of its own
// (the ruling on the review of b23cfe2a3).
// expect: 9:1: error: missing argument for parameter value of constructor SuppressWarnings in trait SuppressWarnings: (value: Array[String]): SuppressWarnings
// expect: 10:36: error: parameter since of constructor Deprecated in trait Deprecated: (since: String, forRemoval: Boolean): Deprecated is already instantiated
@java.lang.Deprecated def a = 1
@java.lang.Deprecated(since = "1") def b = 2
@java.lang.SuppressWarnings(Array("unchecked")) def c = 3
@java.lang.SuppressWarnings() def d = 4
@java.lang.Deprecated(since = "a", since = "b") def e = 5
@java.lang.FunctionalInterface trait T { def f(): Unit }
object Test { def main(args: Array[String]): Unit = println(a + b + c + d) }
