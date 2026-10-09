// expect: a @jsImport definition cannot have a body
// expect: @jsImport is only allowed on top-level definitions
// expect: a @jsImport definition cannot have using clauses
// expect: @jsImport needs a module and a name: @jsImport("module", "name")
// expect: @jsImport is only allowed on a def or a val
// expect: default arguments are not supported on @jsImport definitions
// expect: @js and @jsImport cannot be combined
// expect: @jsExport is only allowed on top-level definitions
// expect: @jsExport needs the exported name: @jsExport("name")
// expect: "not-a-name" is not a valid export name
// expect: a @jsExport definition cannot have using clauses
// expect: a @jsExport definition needs a body
// expect: the repeated parameter of a @jsExport def has to come last
// expect: @jsExport is only allowed on a def or a val
// expect: twice is exported twice
// expect: by-name parameters are not supported on @jsImport definitions
// expect: a definition can only have one @jsImport
// expect: forgotten needs a body; only @js and @jsImport definitions go without one
// expect: forgottenVal needs a body; only @js and @jsImport definitions go without one

@jsImport("node:path", "join")
def withBody(a: String): String = a

@jsImport("node:path", "sep")
val withInitializer: String = "/"

object Holder:
  @jsImport("node:path", "join")
  def nested(a: String): String

  @jsExport("nestedExport")
  def nestedExport(): Int = 1

@jsImport("node:path", "join")
def withUsing(a: String)(using ord: Ordering[String]): String

@jsImport("node:path", "join")
def withContextBound[T: Ordering](a: T): String

@jsImport("node:path")
def oneArgument(a: String): String

@jsImport("node:path", "sep")
var mutableImport: String

@jsImport("node:path", "join")
class ImportedClass

@jsImport("node:path", "join")
def withDefault(a: String = "x"): String

@js("$0") @jsImport("node:path", "join")
def intrinsicToo(a: String): String

@jsExport
def noName(): Int = 1

@jsExport("not-a-name")
def badName(): Int = 1

@jsExport("usingExport")
def usingExport(x: Int)(using ord: Ordering[Int]): Int = x

@jsExport("bodyless")
def bodyless(x: Int): Int

@jsExport("varargsFirst")
def varargsFirst(xs: Int*)(y: Int): Int = y

@jsExport("anObject")
object Exported

@jsExport("twice")
def first(): Int = 1

@jsExport("twice")
def second(): Int = 2

def local(): String =
  @jsImport("node:path", "join")
  def inner(a: String): String
  inner("a")

@jsImport("node:util", "inspect")
def byName(value: => Any): String

@jsImport("node:path", "join") @jsImport("node:path", "resolve")
def importedTwice(a: String): String

// a misspelled annotation must not end in a ReferenceError at run time
@jsimport("node:path", "join")
def forgotten(a: String): String

val forgottenVal: String
