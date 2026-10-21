/*
rules = DisableSyntax
DisableSyntax.noCovariantTypes = true
DisableSyntax.noContravariantTypes = true
*/
// The parameters of a higher-kinded parameter's clause and their variances, nested ones too, are in the tree (dotty's
// `typeParamClause` keeps the clause): each `+` and `-` is reported (the review of `a24540c28`).
package test.disableSyntax

trait Higher[F[+A]] // assert: DisableSyntax.covariant
trait Contra[G[-B], H[_]] // assert: DisableSyntax.contravariant
trait Nested[K[L[+C]]] // assert: DisableSyntax.covariant
