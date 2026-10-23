// Program 498 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 868, "anonymous, func arg
// types, coloneol"): scalac 3.8.4's parser accepts it, teq's reports "expected ':', found ')'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 70
//     Defn.Object 0 69
//       Term.Name 7 11
//       Template 12 69
//         Template.Body 12 69
//           Defn.Given 14 66
//             Name.Anonymous 20 20
//             Member.ParamClauseGroup 20 36
//               Type.ParamClause 20 20
//               Term.ParamClause 20 23
//                 Term.Param 21 22
//                   Name.Anonymous 21 21
//                   Type.Name 21 22
//               Term.ParamClause 27 33
//                 Term.Param 27 33
//                   Name.Anonymous 27 27
//                   Type.Apply 27 33
//                     Type.Name 27 30
//                     Type.ArgClause 30 33
//                       Type.Name 31 32
//             Template 37 66
//               Init 37 49
//                 Type.Apply 37 49
//                   Type.Name 37 40
//                   Type.ArgClause 40 49
//                     Type.Apply 41 48
//                       Type.Name 41 45
//                       Type.ArgClause 45 48
//                         Type.Name 46 47
//                 Name.Anonymous 49 49
//               Template.Body 49 66
//                 Defn.Def 53 66
//                   Term.Name 57 60
//                   Term.Name 63 66
object W498 {
given (A) => Ord[A] => Ord[List[A]]:
  def foo = ???

}
