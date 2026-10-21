// A match alias of an object named inside it is printed with the object's prefix (`O.Elem[B]`: dotty's `TypeRef`
// keeps its `ThisType` prefix, which the printer shows as `O.`); teq keeps no prefix on `Type::Alias` and prints
// `Elem[B]` (master's). The verdicts are scalac's (the review of `4f8b8bb39`'s item 4, the probe AliasInside).
// expect: 9:14: error: illegal cyclic type reference: lower bound O.Elem[B] of type B refers back to the type itself
// expect: 10:14: error: illegal cyclic type reference: lower bound O.Elem[B] of type B refers back to the type itself
object O {
  type Elem[X] = X match { case List[t] => Elem[t]; case _ => X }
  type AA = [B <: Elem[B]] =>> Int
  type AB = [B >: Elem[B]] =>> Int
  type AC = [B >: Elem[List[List[B]]]] =>> Int
}
