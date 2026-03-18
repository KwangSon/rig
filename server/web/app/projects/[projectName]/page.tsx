"use client";

import { useState, useEffect } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";

const API_BASE = "http://localhost:3000/api/v1";

interface Project {
  name: string;
  owner_id: string;
  owner_name?: string;
  clone_url_http?: string;
}

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  locked_by: string | null;
}

export default function ProjectPage() {
  const params = useParams<{ projectName: string }>();
  const projectName = params.projectName;

  const [project, setProject] = useState<Project | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [currentPath, setCurrentPath] = useState("");
  const [isLoadingFiles, setIsLoadingFiles] = useState(false);
  const [copySuccess, setCopySuccess] = useState(false);

  useEffect(() => {
    const t = localStorage.getItem("token");
    fetchProjectDetails(t);
  }, [projectName]);

  const fetchProjectDetails = async (authToken: string | null) => {
    try {
      const res = await fetch(`${API_BASE}/projects/${projectName}`, {
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
      });

      if (res.ok) {
        const data: Project = await res.json();
        const ownerName = data.owner_name || "User";
        data.clone_url_http = `http://localhost:3000/${ownerName}/${data.name}`;
        setProject(data);

        // Check if current user is owner (simplified admin check for UI)
        if (authToken) {
          const userRes = await fetch(`${API_BASE}/users/me`, {
            headers: { Authorization: `Bearer ${authToken}` },
          });
          if (userRes.ok) {
            const userData = await userRes.json();
            setIsAdmin(data.owner_id === userData.id);
          }
        }

        fetchFiles(authToken, "");
      }
    } catch (e) {
      console.error("Failed to fetch project", e);
    }
  };

  const fetchFiles = async (authToken: string | null, path: string) => {
    setIsLoadingFiles(true);
    try {
      const url = new URL(`${API_BASE}/projects/${projectName}/files`);
      if (path) url.searchParams.append("path", path);

      const res = await fetch(url.toString(), {
        headers: authToken ? { Authorization: `Bearer ${authToken}` } : {},
      });

      if (res.ok) {
        setFiles(await res.json());
        setCurrentPath(path);
      }
    } catch (e) {
      console.error("Failed to load files", e);
    } finally {
      setIsLoadingFiles(false);
    }
  };

  const navigateToDirectory = (path: string) => {
    const t = localStorage.getItem("token");
    fetchFiles(t, path);
  };

  const handleBreadcrumbClick = (index: number) => {
    const parts = currentPath.split("/").filter(Boolean);
    const newPath = parts.slice(0, index + 1).join("/");
    navigateToDirectory(newPath);
  };

  const copyCloneCommand = () => {
    if (project?.clone_url_http) {
      navigator.clipboard.writeText(`rig clone ${project.clone_url_http}`);
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    }
  };

  if (!project) {
    return (
      <div className="flex min-h-[calc(100vh-56px)] items-center justify-center bg-gray-50">
        <div className="h-10 w-10 animate-spin rounded-full border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-[calc(100vh-56px)] bg-gray-50">
      {/* Gitea-style Breadcrumb Header */}
      <div className="border-b border-gray-200 bg-white">
        <div className="mx-auto max-w-7xl px-4 py-6 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3 text-2xl">
              <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-indigo-600 font-bold text-white">
                {project.owner_name?.charAt(0).toUpperCase() || "P"}
              </div>
              <div className="flex items-center gap-1 font-semibold">
                <Link
                  href={`/users/${project.owner_id}`}
                  className="text-indigo-600 hover:underline"
                >
                  {project.owner_name}
                </Link>
                <span className="text-gray-400">/</span>
                <span className="text-gray-900">{project.name}</span>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <button className="flex items-center gap-1.5 rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm font-medium text-gray-700 shadow-sm transition-colors hover:bg-gray-50">
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
                    d="M11.049 2.927c.3-.921 1.603-.921 1.902 0l1.519 4.674a1 1 0 00.95.69h4.915c.969 0 1.371 1.24.588 1.81l-3.976 2.888a1 1 0 00-.363 1.118l1.518 4.674c.3.922-.755 1.688-1.538 1.118l-3.976-2.888a1 1 0 00-1.176 0l-3.976 2.888c-.783.57-1.838-.197-1.538-1.118l1.518-4.674a1 1 0 00-.363-1.118l-3.976-2.888c-.784-.57-.382-1.81.588-1.81h4.914a1 1 0 00.951-.69l1.519-4.674z"
                  />
                </svg>
                Star
              </button>
            </div>
          </div>

          <div className="mt-6 -mb-6 flex space-x-8 overflow-x-auto border-b border-transparent">
            <Link
              href={`/projects/${projectName}`}
              className="flex items-center gap-2 border-b-2 border-indigo-600 pb-3 text-sm font-bold text-gray-900"
            >
              <svg
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"
                />
              </svg>
              Code
            </Link>
            <Link
              href={`/projects/${projectName}/issues`}
              className="flex items-center gap-2 pb-3 text-sm font-medium text-gray-500 transition-colors hover:text-gray-700"
            >
              <svg
                className="h-4 w-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              Issues
            </Link>
            {isAdmin && (
              <Link
                href={`/projects/${projectName}/settings`}
                className="flex items-center gap-2 pb-3 text-sm font-medium text-gray-500 transition-colors hover:text-gray-700"
              >
                <svg
                  className="h-4 w-4"
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
            )}
          </div>
        </div>
      </div>

      <div className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
        {/* Clone Bar */}
        <div className="mb-8 flex items-center gap-4 rounded-lg border border-gray-200 bg-white p-1 shadow-sm">
          <div className="ml-1 flex rounded bg-gray-100 px-3 py-1.5 text-sm font-bold tracking-tight text-gray-700">
            HTTP
          </div>
          <div className="flex-1 truncate px-2 font-mono text-sm text-gray-600">
            rig clone {project.clone_url_http}
          </div>
          <button
            onClick={copyCloneCommand}
            className={`mr-1 flex items-center gap-2 rounded-md px-4 py-1.5 text-sm font-semibold transition-all ${
              copySuccess
                ? "bg-green-600 text-white"
                : "bg-indigo-600 text-white hover:bg-indigo-700 active:scale-95"
            }`}
          >
            {copySuccess ? (
              <>
                <svg
                  className="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M5 13l4 4L19 7"
                  />
                </svg>
                Copied!
              </>
            ) : (
              <>
                <svg
                  className="h-4 w-4"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M8 5H6a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2v-1M8 5a2 2 0 002 2h2a2 2 0 002-2M8 5a2 2 0 012-2h2a2 2 0 012 2m0 0h2a2 2 0 012 2v3m2 4H10m0 0l3-3m-3 3l3 3"
                  />
                </svg>
                Copy Clone Command
              </>
            )}
          </button>
        </div>

        {/* File Explorer Table */}
        <div className="overflow-hidden rounded-lg border border-gray-200 bg-white shadow-sm">
          <div className="flex items-center justify-between border-b border-gray-200 bg-gray-50 px-4 py-3">
            <div className="flex items-center gap-1 text-sm font-semibold text-gray-700">
              <button
                onClick={() => navigateToDirectory("")}
                className="text-indigo-600 hover:underline"
              >
                {project.name}
              </button>
              {currentPath
                .split("/")
                .filter(Boolean)
                .map((part, index) => (
                  <span key={index} className="flex items-center">
                    <span className="mx-1 font-normal text-gray-400">/</span>
                    <button
                      onClick={() => handleBreadcrumbClick(index)}
                      className="text-indigo-600 hover:underline"
                    >
                      {part}
                    </button>
                  </span>
                ))}
            </div>
            {isLoadingFiles && (
              <div className="h-4 w-4 animate-spin rounded-full border-b-2 border-indigo-600"></div>
            )}
          </div>

          <div className="divide-y divide-gray-100">
            {currentPath !== "" && (
              <div
                className="group flex cursor-pointer items-center gap-3 px-4 py-2.5 text-sm transition-colors hover:bg-gray-50"
                onClick={() => {
                  const parts = currentPath.split("/").filter(Boolean);
                  parts.pop();
                  navigateToDirectory(parts.join("/"));
                }}
              >
                <svg
                  className="h-5 w-5 text-gray-400 transition-colors group-hover:text-indigo-600"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6"
                  />
                </svg>
                <span className="font-medium text-gray-500">..</span>
              </div>
            )}

            {files.map((file) => (
              <div
                key={file.path}
                className="group flex items-center justify-between border-l-4 border-transparent px-4 py-3 text-sm transition-colors hover:border-indigo-500 hover:bg-gray-50"
              >
                <div
                  className={`flex flex-1 cursor-pointer items-center gap-3`}
                  onClick={() => file.is_dir && navigateToDirectory(file.path)}
                >
                  {file.is_dir ? (
                    <svg
                      className="h-5 w-5 text-indigo-400 transition-colors group-hover:text-indigo-600"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path d="M2 6a2 2 0 012-2h4l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                    </svg>
                  ) : (
                    <svg
                      className="h-5 w-5 text-gray-400 transition-colors group-hover:text-indigo-500"
                      fill="currentColor"
                      viewBox="0 0 20 20"
                    >
                      <path
                        fillRule="evenodd"
                        d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4zm2 6a1 1 0 011-1h6a1 1 0 110 2H7a1 1 0 01-1-1zm1 3a1 1 0 100 2h6a1 1 0 100-2H7z"
                        clipRule="evenodd"
                      />
                    </svg>
                  )}
                  <span
                    className={`font-medium ${file.is_dir ? "text-gray-900 group-hover:text-indigo-600" : "text-gray-700"}`}
                  >
                    {file.name}
                  </span>
                </div>

                <div className="flex items-center gap-4">
                  {file.locked_by && (
                    <div className="flex items-center gap-1.5 rounded border border-red-100 bg-red-50 px-2 py-1 text-xs font-bold text-red-700 shadow-sm">
                      <svg
                        className="h-3.5 w-3.5"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                      >
                        <path
                          fillRule="evenodd"
                          d="M5 9V7a5 5 0 0110 0v2a2 2 0 012 2v5a2 2 0 01-2 2H5a2 2 0 01-2-2v-5a2 2 0 012-2zm8-2v2H7V7a3 3 0 016 0z"
                          clipRule="evenodd"
                        />
                      </svg>
                      LOCKED BY {file.locked_by.toUpperCase()}
                    </div>
                  )}
                  <div className="text-xs font-medium text-gray-400 opacity-0 transition-opacity group-hover:opacity-100">
                    {file.is_dir ? "Directory" : "File"}
                  </div>
                </div>
              </div>
            ))}

            {files.length === 0 && !isLoadingFiles && (
              <div className="px-4 py-12 text-center">
                <svg
                  className="mx-auto mb-3 h-12 w-12 text-gray-300"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={1}
                    d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9l-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"
                  />
                </svg>
                <p className="font-medium text-gray-500">
                  This folder is empty.
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
