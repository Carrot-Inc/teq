// Program 821 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 976, "class Baz1 @deprecated(c:
// C)"): scalac 3.8.4's parser accepts it, teq's reports "expected 'extends' or a template body, found '@'". The
// expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start
// and end in UTF-16 units from the program's first line below.
//   Source 0 45
//     Defn.Object 0 44
//       Term.Name 7 11
//       Template 12 44
//         Template.Body 12 44
//           Defn.Class 14 42
//             Type.Name 20 24
//             Type.ParamClause 25 25
//             Ctor.Primary 25 42
//               Mod.Annot 25 36
//                 Init 26 36
//                   Type.Name 26 36
//                   Name.Anonymous 36 36
//               Name.Anonymous 36 36
//               Term.ParamClause 36 42
//                 Term.Param 37 41
//                   Term.Name 37 38
//                   Type.Name 40 41
//             Template 42 42
//               Template.Body 42 42
object W821 {
class Baz1 @deprecated(c: C)
}
