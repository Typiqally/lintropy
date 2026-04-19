import React from "react";

type Props = {
  title: string;
  payload: any;
};

export function Card({ title, payload }: Props) {
  console.log("rendering card", title);
  return (
    <div className="card">
      <h2>{title}</h2>
      <pre>{JSON.stringify(payload)}</pre>
    </div>
  );
}
