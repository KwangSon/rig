"use client";

import { useEffect, useState, useRef } from "react";
import Link from "next/link";

interface User {
  id: string;
  name: string;
  email: string;
  role: string;
}

export default function Navbar() {
  const [isLoggedIn, setIsLoggedIn] = useState(false);
  const [user, setUser] = useState<User | null>(null);
  const [createDropdownOpen, setCreateDropdownOpen] = useState(false);
  const [userDropdownOpen, setUserDropdownOpen] = useState(false);

  const createRef = useRef<HTMLDivElement>(null);
  const userRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const token = localStorage.getItem("token");
    if (token) {
      setIsLoggedIn(true);
      fetchUser(token);
    }

    const handleClickOutside = (event: MouseEvent) => {
      if (
        createRef.current &&
        !createRef.current.contains(event.target as Node)
      ) {
        setCreateDropdownOpen(false);
      }
      if (userRef.current && !userRef.current.contains(event.target as Node)) {
        setUserDropdownOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const fetchUser = async (token: string) => {
    try {
      const res = await fetch("http://localhost:3000/api/v1/users/me", {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        setUser(await res.json());
      } else {
        // Token invalid
        handleLogout();
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleLogout = () => {
    localStorage.removeItem("token");
    setIsLoggedIn(false);
    setUser(null);
    window.location.href = "/";
  };

  return (
    <nav className="sticky top-0 z-50 border-b border-gray-200 bg-white">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-14 items-center justify-between text-sm">
          {/* Left section */}
          <div className="flex items-center space-x-6">
            <Link href="/" className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded bg-indigo-600 text-xl font-black text-white transition-colors hover:bg-indigo-500">
                R
              </div>
              <span className="text-xl font-bold text-gray-900">Rig</span>
            </Link>
            <div className="mt-1 hidden items-center space-x-4 sm:flex">
              <Link
                href="/explore"
                className="font-medium text-gray-600 transition-colors hover:text-indigo-600"
              >
                Explore
              </Link>
              <Link
                href="/help"
                className="font-medium text-gray-600 transition-colors hover:text-indigo-600"
              >
                Help
              </Link>
            </div>
          </div>

          {/* Right section */}
          <div className="flex items-center space-x-3">
            {isLoggedIn ? (
              <>
                {/* Create Dropdown */}
                <div className="relative" ref={createRef}>
                  <button
                    onClick={() => {
                      setCreateDropdownOpen(!createDropdownOpen);
                      setUserDropdownOpen(false);
                    }}
                    className="flex items-center gap-1 rounded border border-transparent bg-white px-2 py-1.5 text-gray-700 transition-colors hover:border-gray-200 hover:bg-gray-100"
                    title="Create"
                  >
                    <svg
                      className="h-5 w-5"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M12 4v16m8-8H4"
                      />
                    </svg>
                    <svg
                      className="h-4 w-4 text-gray-400"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M19 9l-7 7-7-7"
                      />
                    </svg>
                  </button>

                  {createDropdownOpen && (
                    <div className="ring-opacity-5 absolute right-0 mt-2 w-48 origin-top-right rounded-md bg-white py-1 shadow-lg ring-1 ring-black focus:outline-none">
                      <Link
                        href="/projects/new"
                        className="flex items-center px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                        onClick={() => setCreateDropdownOpen(false)}
                      >
                        <svg
                          className="mr-3 h-4 w-4 text-gray-400"
                          fill="none"
                          viewBox="0 0 24 24"
                          stroke="currentColor"
                        >
                          <path
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            strokeWidth={2}
                            d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
                          />
                        </svg>
                        New Project
                      </Link>
                    </div>
                  )}
                </div>

                {/* User Dropdown */}
                <div className="relative" ref={userRef}>
                  <button
                    onClick={() => {
                      setUserDropdownOpen(!userDropdownOpen);
                      setCreateDropdownOpen(false);
                    }}
                    className="flex items-center gap-2 rounded-full border border-transparent p-1 pr-2 transition-colors hover:border-gray-200 hover:bg-gray-50 focus:outline-none"
                  >
                    <div className="flex h-7 w-7 items-center justify-center rounded bg-indigo-100 font-bold text-indigo-700">
                      {user?.name?.charAt(0).toUpperCase() || "U"}
                    </div>
                    <svg
                      className="h-4 w-4 text-gray-500"
                      fill="none"
                      viewBox="0 0 24 24"
                      stroke="currentColor"
                    >
                      <path
                        strokeLinecap="round"
                        strokeLinejoin="round"
                        strokeWidth={2}
                        d="M19 9l-7 7-7-7"
                      />
                    </svg>
                  </button>

                  {userDropdownOpen && (
                    <div className="ring-opacity-5 absolute right-0 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white py-1 shadow-lg ring-1 ring-black focus:outline-none">
                      <div className="px-4 py-3">
                        <p className="text-sm">Signed in as</p>
                        <p className="truncate text-sm font-medium text-gray-900">
                          {user?.name || user?.email}
                        </p>
                      </div>
                      <div className="py-1">
                        <Link
                          href="/settings"
                          className="flex px-4 py-2 text-sm text-gray-700 hover:bg-gray-100"
                          onClick={() => setUserDropdownOpen(false)}
                        >
                          Settings
                        </Link>
                      </div>
                      <div className="py-1">
                        <button
                          onClick={handleLogout}
                          className="flex w-full px-4 py-2 text-left text-sm text-gray-700 hover:bg-gray-100"
                        >
                          Sign out
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              </>
            ) : (
              <>
                <Link
                  href="/auth/login"
                  className="rounded-md px-3 py-2 font-medium text-gray-700 transition-colors hover:bg-gray-50 hover:text-indigo-600"
                >
                  Sign in
                </Link>
                <Link
                  href="/auth/signup"
                  className="rounded-md bg-indigo-600 px-3 py-1.5 font-medium text-white shadow-sm transition-all hover:bg-indigo-500"
                >
                  Sign up
                </Link>
              </>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
}
