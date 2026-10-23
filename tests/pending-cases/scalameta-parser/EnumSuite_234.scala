// Program 234 of tests/fix/scalameta, from scalameta 4.17.4's EnumSuite (line 23, "enum-private-case"): scalac
// 3.8.4's parser accepts it, teq's reports "expected a definition, found 'case'". The expectation is scalameta's
// tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in UTF-16 units from
// the program's first line below.
//   Source 0 64
//     Defn.Object 0 63
//       Term.Name 7 11
//       Template 12 63
//         Template.Body 12 63
//           Defn.Enum 14 61
//             Type.Name 19 24
//             Type.ParamClause 25 25
//             Ctor.Primary 25 25
//               Name.Anonymous 25 25
//             Template 25 61
//               Template.Body 25 61
//                 Defn.EnumCase 27 41
//                   Mod.Private 27 34
//                     Name.Anonymous 35 35
//                   Term.Name 40 41
//                   Type.ParamClause 41 41
//                   Ctor.Primary 41 41
//                     Name.Anonymous 41 41
//                 Defn.EnumCase 43 59
//                   Mod.Protected 43 52
//                     Name.Anonymous 53 53
//                   Term.Name 58 59
//                   Type.ParamClause 60 60
//                   Ctor.Primary 60 60
//                     Name.Anonymous 60 60
object W234 {
enum Color { private case R; protected case G }
}
