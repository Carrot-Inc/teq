// expect: jvm backend: Main$.main: the JavaScript template $getClass($0)
// expect: jvm backend: Main$.main: the JavaScript template $productPrefix($0)
// expect: jvm backend: Main$.main: the JavaScript template $hashCode($0)
// expect: jvm backend: Main$.main: the JavaScript template $equals($0, $1)
// expect: jvm backend: Main$.main: the JavaScript template $identityHash($0)
// expect: jvm backend: Main$.main: the JavaScript template $0.$ordinal
// expect: jvm backend: Main$.main: the JavaScript template $synchronized($0, $1)
// expect: 7 constructs not supported by the JVM backend
// teq: --target jvm
// command: build
// exit: 3
// A program's own `jvm` annotation can write the typer's own texts on a top-level def, whose
// parameters start at $0: with fewer arguments than the text names, each is refused where it
// is lowered, as a text with no JVM translation, never written by the old generator.
class jvm(template: String) extends scala.annotation.StaticAnnotation

@jvm("$getClass($0)") def getClassOf(): Object = ???
@jvm("$productPrefix($0)") def prefixOf(): String = ???
@jvm("$hashCode($0)") def hashOf(): Int = ???
@jvm("$equals($0, $1)") def equalTo(a: Any): Boolean = ???
@jvm("$identityHash($0)") def identityOf(): Int = ???
@jvm("$0.$ordinal") def ordinalOf(): Int = ???
@jvm("$synchronized($0, $1)") def lockedOn(a: Any): Any = ???

object Main:
  def main(args: Array[String]): Unit =
    println(getClassOf())
    println(prefixOf())
    println(hashOf())
    println(equalTo(1))
    println(identityOf())
    println(ordinalOf())
    println(lockedOn(1))
