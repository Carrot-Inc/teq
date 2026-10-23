// Program 493 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 784, "named, param clause
// first, braces"): scalac 3.8.4's parser reads it as scalameta does, `given ord with clauses (a: A)(using Ord[A]) of
// parent Ord[List[A]]`, and teq's otherwise. The expectation is scalameta's tree under dialects.Scala3, which teq
// fix's tree answers to: a node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 81
//     Defn.Object 0 80
//       Term.Name 7 11
//       Template 12 80
//         Template.Body 12 80
//           Defn.Given 14 77
//             Term.Name 20 23
//             Member.ParamClauseGroup 25 44
//               Type.ParamClause 25 25
//               Term.ParamClause 25 31
//                 Term.Param 26 30
//                   Term.Name 26 27
//                   Type.Name 29 30
//               Term.ParamClause 35 41
//                 Term.Param 35 41
//                   Name.Anonymous 35 35
//                   Type.Apply 35 41
//                     Type.Name 35 38
//                     Type.ArgClause 38 41
//                       Type.Name 39 40
//             Template 45 77
//               Init 45 57
//                 Type.Apply 45 57
//                   Type.Name 45 48
//                   Type.ArgClause 48 57
//                     Type.Apply 49 56
//                       Type.Name 49 53
//                       Type.ArgClause 53 56
//                         Type.Name 54 55
//                 Name.Anonymous 58 58
//               Template.Body 58 77
//                 Defn.Def 62 75
//                   Term.Name 66 69
//                   Term.Name 72 75
object W493 {
given ord: (a: A) => Ord[A] => Ord[List[A]] {
  def foo = ???
}

}
