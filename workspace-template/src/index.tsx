import React from "react";
import ReactDOM from "react-dom/client";
import { Player } from "@remotion/player";
import { Main } from "./Composition";

const Root = () => (
  <div style={{ width: "100vw", height: "100vh", margin: 0 }}>
    <Player
      component={Main}
      durationInFrames={150}
      fps={30}
      compositionWidth={1920}
      compositionHeight={1080}
      style={{ width: "100%", height: "100%" }}
      controls
    />
  </div>
);

ReactDOM.createRoot(document.getElementById("root")!).render(<Root />);
