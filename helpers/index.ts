import { readFile, writeFile } from "node:fs/promises";
import {
  Document,
  Packer,
  Paragraph,
  TextRun,
  PageBreak,
  Footer,
  AlignmentType,
  type IRunOptions,
  type ISectionOptions,
} from "docx";
import data from "../output.json";
import yaml from "js-yaml";

async function sections(): Promise<ISectionOptions[]> {
  const children: Paragraph[] = [];
  const doc = yaml.load(await readFile("format.yml", "utf-8")) as SchemaType;
  for (const problem of data.sort((a, b) => a.index - b.index)) {
    const parsed = (await format(
      doc,
      problem.question,
      problem.code,
      problem.output,
      data.indexOf(problem) + 1
    ))!;

    children.push(
      new Paragraph({
        children: [new TextRun(parsed.question)],
      })
    );

    children.push(
      new Paragraph({
        children: [new TextRun(parsed.solution.title)],
      })
    );

    // Word and Docs cannot handle line breaks.
    const codeLines = parsed.solution.text!.split("\n");
    codeLines.forEach((line) => {
      children.push(
        new Paragraph({
          children: [
            new TextRun({
              text: line,
              size: parsed.output.size,
            }),
          ],
        })
      );
    });

    children.push(
      new Paragraph({
        children: [new TextRun(parsed.output.title)],
      })
    );

    // Word and Docs cannot handle line breaks.
    const outputLines = parsed.output.text!.split("\n");
    for (const line of outputLines) {
      children.push(
        new Paragraph({
          children: [
            new TextRun({
              text: line,
              size: parsed.output.size,
            }),
          ],
        })
      );
    }

    // Page Break
    children.push(new Paragraph({ children: [new PageBreak()] }));
  }
  return [{
    children,
    footers: {
        default: new Footer({
          children: [
            new Paragraph({
              alignment: AlignmentType.RIGHT,
              children: [
                new TextRun({
                  text: "Made by H",
                  size: "10pt",
                }),
              ],
            }),
          ],
        }),
      },
  }];
}

function replacer(text: string, variables: Variables): string {
  return text.replace(/\{([^}]+)\}/g, (_match: string, varName: string) => {
    const value = variables[varName];
    return value !== undefined ? String(value) : "";
  });
}

async function format(
  doc: SchemaType,
  question: string,
  solution: string,
  output: string,
  n: number
) {
  try {
    for (const v of Object.values(doc)) {
      v.text = replacer(v.text, {
        question,
        solution,
        output,
        n,
	  });
    }
	return doc;
  } catch (e) {
    console.error(e);
  }
}

const doc = new Document({
  sections: await sections()
});

Packer.toBuffer(doc).then((buffer) => {
  writeFile("Labfile.docx", buffer);
});

interface SchemaType {
  header?: IRunOptions;
  footer?: IRunOptions;
  question: IRunOptions;
  solution: Extras;
  output: Extras;
}

interface Extras extends IRunOptions {
  title: IRunOptions;
  alignment?:
  | "start"
  | "center"
  | "end"
  | "both"
  | "mediumKashida"
  | "distribute"
  | "numTab"
  | "highKashida"
  | "lowKashida"
  | "thaiDistribute"
  | "left"
  | "right";
}

type Variables = {
  [key: string]: string | number;
};
