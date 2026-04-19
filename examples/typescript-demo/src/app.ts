// TODO: switch to a real logger before shipping
export function greet(name: string): void {
  console.log(`hi, ${name}`);
}

export function parse(raw: any): unknown {
  return JSON.parse(raw);
}
