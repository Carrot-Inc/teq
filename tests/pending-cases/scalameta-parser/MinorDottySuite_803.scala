// Program 803 of tests/fix/scalameta, from scalameta 4.17.4's MinorDottySuite (line 612, "empty-case-class"): scalac
// 3.8.4's parser accepts it, teq's reports "expected 'extends' or a template body, found 'private'". The expectation
// is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 40
//     Defn.Object 0 39
//       Term.Name 7 11
//       Template 12 39
//         Template.Body 12 39
//           Defn.Class 14 37
//             Mod.Case 14 18
//             Type.Name 25 26
//             Type.ParamClause 26 29
//               Type.Param 27 28
//                 Type.Name 27 28
//                 Type.ParamClause 28 28
//                 Type.Bounds 28 28
//             Ctor.Primary 30 37
//               Mod.Private 30 37
//                 Name.Anonymous 38 38
//               Name.Anonymous 38 38
//             Template 38 38
//               Template.Body 38 38
object W803 {
case class A[T] private
}
