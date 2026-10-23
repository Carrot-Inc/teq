// Program 823 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 1005, "class Baz1
// @deprecated(foo)(c: C)"): scalac 3.8.4's parser accepts it, teq's reports "expected 'extends' or a template body,
// found '@'". The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a
// line, its start and end in UTF-16 units from the program's first line below.
//   Source 0 50
//     Defn.Object 0 49
//       Term.Name 7 11
//       Template 12 49
//         Template.Body 12 49
//           Defn.Class 14 47
//             Type.Name 20 24
//             Type.ParamClause 25 25
//             Ctor.Primary 25 47
//               Mod.Annot 25 41
//                 Init 26 41
//                   Type.Name 26 36
//                   Name.Anonymous 36 36
//                   Term.ArgClause 36 41
//                     Term.Name 37 40
//               Name.Anonymous 41 41
//               Term.ParamClause 41 47
//                 Term.Param 42 46
//                   Term.Name 42 43
//                   Type.Name 45 46
//             Template 47 47
//               Template.Body 47 47
object W823 {
class Baz1 @deprecated(foo)(c: C)
}
