// A binder's annotation is typed as dotty's `Typer.completeAnnotations` types a `TypeDef`'s, where `annotContext`
// places it (the review of `16412b2e9`): `checkAnnotClass`, `checkAnnotArgs` and `checkAnnotApplicable` after it is
// typed, a Java annotation's arguments named (`checkNamedArgumentForJavaAnnotation`), its element's name checked; a
// lambda's binders not in scope, nor a parameterized alias's parameters, a type definition's own clause's binders in
// scope, a parenthesized lambda no definition's own (a `Parens`). scalac 3.8.4 gives these fifteen errors, each where teq
// gives it.
// expect: 27:23: error: class Plain is not a valid Scala annotation: it does not extend `scala.annotation.Annotation`
// expect: 28:18: error: main annotation cannot be applied to type A
// expect: 29:29: error: target name cannot be empty
// expect: 31:30: error: @targetName needs a string literal as argument
// expect: 32:54: error: @implicitNotFound requires constant expressions as a parameter
// expect: 34:20: error: class Object is not a valid Java annotation: it was not declared with `@interface`
// expect: 35:32: error: Named arguments are required for Java defined annotations
// expect: 36:29: error: constructor Deprecated in trait Deprecated: (since: String, forRemoval: Boolean): Deprecated does not have a parameter foo
// expect: 38:31: error: type B not found
// expect: 39:24: error: type B not found
// expect: 40:26: error: type B not found
// expect: 42:26: error: type A not found
// expect: 43:13: error: class Plain is not a valid Scala annotation: it does not extend `scala.annotation.Annotation`
// expect: 44:14: error: class Plain is not a valid Scala annotation: it does not extend `scala.annotation.Annotation`
// expect: 45:32: error: type B not found
// absent: binder_annotation_checks.scala:33:
// absent: binder_annotation_checks.scala:37:
import scala.annotation.{implicitNotFound, targetName, StaticAnnotation}
class Plain
class tagged[T] extends StaticAnnotation
trait NotAnnotation[F[@Plain A]]
trait OnBinder[F[@main A]]
type Literal = [@targetName("") A] =>> Int
final val name = "x"
type Constant = [@targetName(name) A] =>> Int
def local(m: String) = { type L = [@implicitNotFound(m) A] =>> Int; 1 }
type Folded = [@implicitNotFound(name) A] =>> Int
type JavaObject = [@Object A] =>> Int
type Positional = [@Deprecated("x") A] =>> Int
type Unknown = [@Deprecated(foo = "x") A] =>> Int
type Value = [@SuppressWarnings(Array("a")) A] =>> Int
type Parameters[B] = [@tagged[B] A] =>> Int
val literal = [@tagged[B] A, B] => (a: A) => a
type Function = [@tagged[B] A, B] => A => A
type M[G[_]] = Int
type Nested = M[[@tagged[A] A] =>> Int]
val poly = [@Plain A] => (a: A) => a
def method[F[@Plain A]] = 1
type Parenthesized = ([@tagged[B] A, B] =>> Int)
