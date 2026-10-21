// A Java annotation of a binder applied as dotty applies its constructor (the review of `50e3a1b38`): its arguments
// ordered by `Applications.reorder` (a duplicate, an unknown name), each element left out its `AnnotationDefault` or
// `MissingArgument` (E171), a positional one `checkNamedArgumentForJavaAnnotation`'s E201 (its own mismatch hidden as
// scalac's reporter hides it). scalac 3.8.4 gives these five errors.
// expect: 14:17: error: missing argument for parameter value of constructor SuppressWarnings in trait SuppressWarnings: (value: Array[String]): SuppressWarnings
// expect: 15:14: error: missing argument for parameter value of constructor SuppressWarnings in trait SuppressWarnings: (value: Array[String]): SuppressWarnings
// expect: 16:54: error: parameter since of constructor Deprecated in trait Deprecated: (since: String, forRemoval: Boolean): Deprecated is already instantiated
// expect: 17:39: error: constructor Deprecated in trait Deprecated: (since: String, forRemoval: Boolean): Deprecated does not have a parameter foo
// expect: 22:50: error: Named arguments are required for Java defined annotations
// absent: binder_java_annotation_args.scala:18:
// absent: binder_java_annotation_args.scala:19:
// absent: binder_java_annotation_args.scala:20:
// absent: binder_java_annotation_args.scala:21:
type Missing = [@java.lang.SuppressWarnings() A] =>> List[A]
type Bare = [@java.lang.SuppressWarnings A] =>> List[A]
type Duplicate = [@java.lang.Deprecated(since = "a", since = "b") A] =>> List[A]
type Unknown = [@java.lang.Deprecated(foo = "x") A] =>> List[A]
type Defaults = [@java.lang.Deprecated() A] =>> List[A]
type BothNames = [@java.lang.Deprecated(forRemoval = true, since = "1") A] =>> List[A]
type Value = [@java.lang.SuppressWarnings(value = Array("a")) A] =>> List[A]
type Lone = [@java.lang.SuppressWarnings(Array("a")) A] =>> List[A]
type Mixed = [@java.lang.Deprecated(since = "1", "x") A] =>> List[A]
