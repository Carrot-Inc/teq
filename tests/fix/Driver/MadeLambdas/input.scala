/*
rules = ["file:../../support/Lambdas.scala"]
*/
// A lambda the parser makes over a higher-kinded parameter's clause is no `Type.Lambda` of the tree: its written bound
// stands under the parameter beside the clause's own (the review of `340567fdd`); a written lambda is one.
package lambdas

trait Upper[X]
trait Higher[F[G[X] <: Upper[X]]]
trait Kinded[H[_]]
type T = [A] =>> List[A] // assert: Lambdas.lambda
