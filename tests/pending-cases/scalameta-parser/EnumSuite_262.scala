// Program 262 of tests/fix/scalameta, from scalameta 4.17.4's EnumSuite (line 372, "enum-self-braces"): scalac
// 3.8.4's parser accepts it, teq's reports "expected end of statement, found 'case'". The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 73
//     Defn.Object 0 72
//       Term.Name 7 11
//       Template 12 72
//         Template.Body 12 72
//           Defn.Enum 14 69
//             Type.Name 19 20
//             Type.ParamClause 21 21
//             Ctor.Primary 21 21
//               Name.Anonymous 21 21
//             Template 21 69
//               Template.Body 21 69
//                 Self 23 30
//                   Term.Name 23 27
//                 Defn.EnumCase 33 47
//                   Term.Name 38 39
//                   Type.ParamClause 39 39
//                   Ctor.Primary 39 47
//                     Name.Anonymous 39 39
//                     Term.ParamClause 39 47
//                       Term.Param 40 46
//                         Term.Name 40 41
//                         Type.Name 43 46
//                 Defn.EnumCase 50 67
//                   Term.Name 55 56
//                   Type.ParamClause 56 56
//                   Ctor.Primary 56 67
//                     Name.Anonymous 56 56
//                     Term.ParamClause 56 67
//                       Term.Param 57 66
//                         Term.Name 57 58
//                         Type.Name 60 66
object W262 {
enum A { self =>
  case B(v: Int)
  case C(v: String)
}

}
