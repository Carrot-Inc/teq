// Program 822 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 988, "class Baz1 @deprecated(c:
// C = some)"): scalac 3.8.4's parser accepts it, teq's reports "expected 'extends' or a template body, found '@'".
// The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 52
//     Defn.Object 0 51
//       Term.Name 7 11
//       Template 12 51
//         Template.Body 12 51
//           Defn.Class 14 49
//             Type.Name 20 24
//             Type.ParamClause 25 25
//             Ctor.Primary 25 49
//               Mod.Annot 25 36
//                 Init 26 36
//                   Type.Name 26 36
//                   Name.Anonymous 36 36
//               Name.Anonymous 36 36
//               Term.ParamClause 36 49
//                 Term.Param 37 48
//                   Term.Name 37 38
//                   Type.Name 40 41
//                   Term.Name 44 48
//             Template 49 49
//               Template.Body 49 49
object W822 {
class Baz1 @deprecated(c: C = some)
}
