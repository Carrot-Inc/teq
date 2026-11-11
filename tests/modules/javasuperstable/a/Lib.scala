// A Java class runs its constructor: only a Java interface is `NoInits` (`JavaInterface`, Flags.scala:601) and
// `Object` is given it by hand (Definitions.scala:324). So a class extending another Java class is no
// `isNoInitsRealClass` (SymDenotations.scala:813-815), and its constructor is not stable (`setStableConstructor`,
// 97-99): a downstream's `isPureApply` (TreeInfo.scala:673) keeps its creation. Its bundle row fails under --release
// alone, the same on push 9's tip: the release renaming's shift
// (tests/pending-cases/products_release_names_completion's mechanism), not traced to a class.
package jsa

class Thr extends Thread

class Jl extends java.util.ArrayList[Int]
