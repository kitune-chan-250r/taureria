import { createHashRouter, RouterProvider } from "react-router";
import { Home } from "./Home";
import { Viewer } from "./Viewer";

export const Router = () => {
  const router = createHashRouter([
    { path: "/", element: <Home /> },
    { path: "/viewer", element: <Viewer /> },
  ]);

  return <RouterProvider router={router} />;
};
