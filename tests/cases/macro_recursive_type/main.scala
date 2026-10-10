// A recursive refinement as reflection reads it (`RecursiveType`, its `underlying` naming its
// members through the `RecursiveThis`, whose `binder` is the type), and one built by
// `RecursiveType.apply`, as dotty's `QuotesImpl` has them; a refinement whose members name no
// sibling is no recursive type.
package app
import mrec.Recursive

type R = AnyRef { type T; type U = T }
type S = AnyRef { type T = String; def f(): T }
type P = AnyRef { type T; def g: Int }

@main def run(): Unit =
  println(Recursive.of[R])
  println(Recursive.of[S])
  println(Recursive.of[P])
