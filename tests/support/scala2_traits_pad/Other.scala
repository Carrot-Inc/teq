// A Scala 3 class that tests/support/jars.sh's scala2-traits-mixed adds, with its TASTy, to the scala2-traits-padded jar:
// a jar of Scala 2 traits, Java classes and a Scala 3 class, each told by its own TASTy or class file
// (tests/classpath/jvm/scala2_traits_mixed).
package unrelated

class Other { def answer: Int = 42 }
