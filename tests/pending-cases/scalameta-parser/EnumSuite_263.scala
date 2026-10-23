// Program 263 of tests/fix/scalameta, from scalameta 4.17.4's EnumSuite (line 399, "enum-self-indented"): scalac
// 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'case'". The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 94
//     Defn.Object 0 93
//       Term.Name 7 11
//       Template 12 93
//         Template.Body 12 93
//           Defn.Enum 14 90
//             Type.Name 19 20
//             Type.ParamClause 20 20
//             Ctor.Primary 20 20
//               Name.Anonymous 20 20
//             Template 20 90
//               Template.Body 20 90
//                 Self 24 31
//                   Term.Name 24 28
//                 Defn.EnumCase 36 50
//                   Term.Name 41 42
//                   Type.ParamClause 42 42
//                   Ctor.Primary 42 50
//                     Name.Anonymous 42 42
//                     Term.ParamClause 42 50
//                       Term.Param 43 49
//                         Term.Name 43 44
//                         Type.Name 46 49
//                 Defn.EnumCase 55 72
//                   Term.Name 60 61
//                   Type.ParamClause 61 61
//                   Ctor.Primary 61 72
//                     Name.Anonymous 61 61
//                     Term.ParamClause 61 72
//                       Term.Param 62 71
//                         Term.Name 62 63
//                         Type.Name 65 71
//                 Defn.Def 75 90
//                   Term.Name 79 81
//                   Type.Name 83 86
//                   Lit.Int 89 90
object W263 {
enum A:
  self =>
    case B(v: Int)
    case C(v: String)
  def fx: Int = 4

}
