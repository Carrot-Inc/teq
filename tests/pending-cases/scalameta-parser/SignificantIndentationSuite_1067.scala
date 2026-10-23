// Program 1067 of tests/fix/scalameta, from scalameta 4.17.4's SignificantIndentationSuite (line 2710, "def newline
// around hash"): scalac 3.8.4's parser reads it as scalameta does, `def foo: Bar#SubBar = baz`, and teq's otherwise.
// The expectation is scalameta's tree under dialects.Scala3, which teq fix's tree answers to: a node a line, its
// start and end in UTF-16 units from the program's first line below.
//   Source 0 54
//     Defn.Object 0 53
//       Term.Name 7 12
//       Template 13 53
//         Template.Body 13 53
//           Defn.Def 15 50
//             Term.Name 19 22
//             Type.Project 24 40
//               Type.Name 24 27
//               Type.Name 34 40
//             Term.Name 47 50
object W1067 {
def foo: Bar
  #
  SubBar =
    baz

}
