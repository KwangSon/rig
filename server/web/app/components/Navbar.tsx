"use client";

import { useEffect, useState } from "react";

export default function Navbar() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);

  useEffect(() => {
    const token = localStorage.getItem("token");
    setIsLoggedIn(!!token);
  }, []);

  const handleLogout = () => {
    localStorage.removeItem("token");
    setIsLoggedIn(false);
    window.location.href = "/";
  };

  return (
    <nav className="border-b bg-white shadow-sm">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-16 justify-between">
          <div className="flex items-center">
            <a href="/" className="text-xl font-bold text-gray-900">
              Rig
            </a>
          </div>
          <div className="flex items-center space-x-4">
            {isLoggedIn ? (
              <button
                onClick={handleLogout}
                className="rounded-md px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900"
              >
                Logout
              </button>
            ) : (
              <>
                <a
                  href="/auth/login"
                  className="rounded-md px-3 py-2 text-sm font-medium text-gray-700 hover:text-gray-900"
                >
                  Sign in
                </a>
                <a
                  href="/auth/signup"
                  className="rounded-md bg-indigo-600 px-3 py-2 text-sm font-medium text-white hover:bg-indigo-700"
                >
                  Sign up
                </a>
              </>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
}
