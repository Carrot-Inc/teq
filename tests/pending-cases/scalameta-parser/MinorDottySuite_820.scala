// Program 820 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 959, "class Baz1
// @deprecated(implicit c: C)"): scalac 3.8.4's parser accepts it, teq's reports "expected 'extends' or a template
// body, found '@'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a
// node a line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 54
//     Defn.Object 0 53
//       Term.Name 7 11
//       Template 12 53
//         Template.Body 12 53
//           Defn.Class 14 51
//             Type.Name 20 24
//             Type.ParamClause 25 25
//             Ctor.Primary 25 51
//               Mod.Annot 25 36
//                 Init 26 36
//                   Type.Name 26 36
//                   Name.Anonymous 36 36
//               Name.Anonymous 36 36
//               Term.ParamClause 36 51
//                 Term.Param 46 50
//                   Mod.Implicit 37 45
//                   Term.Name 46 47
//                   Type.Name 49 50
//                 Mod.Implicit 37 45
//             Template 51 51
//               Template.Body 51 51
object W820 {
class Baz1 @deprecated(implicit c: C)
}
