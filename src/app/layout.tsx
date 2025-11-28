import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Sync2Bucket",
  description: "Sync your folders with cloud storage",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className="antialiased">
        {children}
      </body>
    </html>
  );
}
