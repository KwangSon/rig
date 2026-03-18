"use client";

import { useEffect, useState, useRef } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";

interface User {
  id: string;
  name: string;
  email: string;
}

export default function Navbar() {
  const pathname = usePathname();
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

  const isActive = (path: string) => pathname === path;

  return (
    <nav className="sticky top-0 z-50 border-b border-gray-200 bg-white shadow-sm">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="flex h-14 items-center justify-between">
          {/* Left section */}
          <div className="flex items-center space-x-8">
            <Link href="/" className="group flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded bg-indigo-600 text-xl font-black text-white transition-all group-hover:scale-105 group-hover:bg-indigo-700">
                R
              </div>
              <span className="text-xl font-bold tracking-tight text-gray-900">
                Rig
              </span>
            </Link>

            <div className="hidden items-center space-x-1 sm:flex">
              <Link
                href="/explore"
                className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                  isActive("/explore")
                    ? "bg-indigo-50 text-indigo-700"
                    : "text-gray-600 hover:bg-gray-50 hover:text-indigo-600"
                }`}
              >
                Explore
              </Link>
              <Link
                href="/help"
                className={`rounded-md px-3 py-2 text-sm font-medium transition-colors ${
                  isActive("/help")
                    ? "bg-indigo-50 text-indigo-700"
                    : "text-gray-600 hover:bg-gray-50 hover:text-indigo-600"
                }`}
              >
                Help
              </Link>
            </div>
          </div>

          {/* Right section */}
          <div className="flex items-center space-x-2">
            {isLoggedIn ? (
              <>
                {/* Create Dropdown */}
                <div className="relative" ref={createRef}>
                  <button
                    onClick={() => {
                      setCreateDropdownOpen(!createDropdownOpen);
                      setUserDropdownOpen(false);
                    }}
                    className="flex h-9 w-9 items-center justify-center rounded-md border border-gray-200 bg-white text-gray-600 transition-all hover:border-gray-300 hover:bg-gray-50 focus:outline-none"
                    title="Create new..."
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
                  </button>

                  {createDropdownOpen && (
                    <div className="ring-opacity-5 absolute right-0 z-50 mt-2 w-48 origin-top-right rounded-md bg-white py-1 shadow-lg ring-1 ring-black focus:outline-none">
                      <Link
                        href="/projects/new"
                        className="flex items-center px-4 py-2 text-sm text-gray-700 transition-colors hover:bg-indigo-50 hover:text-indigo-700"
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
                <div className="relative ml-2" ref={userRef}>
                  <button
                    onClick={() => {
                      setUserDropdownOpen(!userDropdownOpen);
                      setCreateDropdownOpen(false);
                    }}
                    className="flex items-center gap-2 rounded-md border border-transparent px-2 py-1 transition-all hover:border-gray-200 hover:bg-gray-50 focus:outline-none"
                  >
                    <div className="flex h-8 w-8 items-center justify-center rounded border border-indigo-200 bg-indigo-100 font-bold text-indigo-700">
                      {user?.name?.charAt(0).toUpperCase() || "U"}
                    </div>
                    <span className="hidden max-w-[100px] truncate text-sm font-medium text-gray-700 md:block">
                      {user?.name}
                    </span>
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

                  {userDropdownOpen && (
                    <div className="ring-opacity-5 absolute right-0 z-50 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white py-1 shadow-lg ring-1 ring-black focus:outline-none">
                      <div className="px-4 py-3">
                        <p className="text-xs font-semibold tracking-wider text-gray-500 uppercase">
                          Signed in as
                        </p>
                        <p className="mt-0.5 truncate text-sm font-bold text-gray-900">
                          {user?.email}
                        </p>
                      </div>
                      <div className="py-1">
                        <Link
                          href="/settings"
                          className="flex px-4 py-2 text-sm text-gray-700 transition-colors hover:bg-indigo-50 hover:text-indigo-700"
                          onClick={() => setUserDropdownOpen(false)}
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
                              d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"
                            />
                            <path
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth={2}
                              d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
                            />
                          </svg>
                          Settings
                        </Link>
                      </div>
                      <div className="py-1">
                        <button
                          onClick={handleLogout}
                          className="flex w-full px-4 py-2 text-left text-sm text-red-600 transition-colors hover:bg-red-50"
                        >
                          <svg
                            className="mr-3 h-4 w-4 text-red-400"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                          >
                            <path
                              strokeLinecap="round"
                              strokeLinejoin="round"
                              strokeWidth={2}
                              d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"
                            />
                          </svg>
                          Sign out
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              </>
            ) : (
              <div className="flex items-center space-x-2">
                <Link
                  href="/auth/login"
                  className="px-4 py-2 text-sm font-medium text-gray-700 transition-colors hover:text-indigo-600"
                >
                  Sign in
                </Link>
                <Link
                  href="/auth/signup"
                  className="rounded-md bg-indigo-600 px-4 py-2 text-sm font-semibold text-white shadow-sm transition-all hover:bg-indigo-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-indigo-600"
                >
                  Sign up
                </Link>
              </div>
            )}
          </div>
        </div>
      </div>
    </nav>
  );
}
