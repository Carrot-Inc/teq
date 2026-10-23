// Program 409 of tests/fix/scalameta, from scalameta 4.17.4's FewerBracesSuite (line 2307, "#4152 not fewer braces:
// given"): scalac 3.8.4's parser accepts it, teq's reports "expected ',' or ')', found ':'". The expectation is
// scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its start and end in
// UTF-16 units from the program's first line below.
//   Source 0 44
//     Defn.Object 0 43
//       Term.Name 7 11
//       Template 12 43
//         Template.Body 12 43
//           Defn.GivenAlias 14 40
//             Name.Anonymous 20 20
//             Type.Tuple 20 36
//               Type.TypedParam 21 35
//                 Type.Name 21 22
//                 Type.Name 32 35
//             Term.Name 39 40
object W409 {
given (b:
        Int) = b

}
