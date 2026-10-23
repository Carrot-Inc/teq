// Program 499 of tests/fix/scalameta, from scalameta 4.17.4's GivenSyntax36Suite (line 885, "anonymous, func arg
// types, braces"): scalac 3.8.4's parser accepts it, teq's reports "expected ':', found ')'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 73
//     Defn.Object 0 72
//       Term.Name 7 11
//       Template 12 72
//         Template.Body 12 72
//           Defn.Given 14 69
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
//             Template 37 69
//               Init 37 49
//                 Type.Apply 37 49
//                   Type.Name 37 40
//                   Type.ArgClause 40 49
//                     Type.Apply 41 48
//                       Type.Name 41 45
//                       Type.ArgClause 45 48
//                         Type.Name 46 47
//                 Name.Anonymous 50 50
//               Template.Body 50 69
//                 Defn.Def 54 67
//                   Term.Name 58 61
//                   Term.Name 64 67
object W499 {
given (A) => Ord[A] => Ord[List[A]] {
  def foo = ???
}

}
