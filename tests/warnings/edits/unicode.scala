package edits.unicode
object O { val α = 1; val β = 2; val γ = 3 }
object Test { import O.{α, β, γ}; val x = α + γ }
